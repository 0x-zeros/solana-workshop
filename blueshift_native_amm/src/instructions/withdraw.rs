use super::helpers::*;
use crate::errors::AmmError;
use crate::state::{AmmState, Config};
use core::mem::size_of;
use constant_product_curve::ConstantProduct;
use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    sysvars::{Sysvar, rent::Rent, clock::Clock},
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::Mint;
use pinocchio_token::{
    instructions::{CloseAccount, Transfer, MintTo, Burn},
    state::TokenAccount,
};

pub struct Withdraw<'a> {
    pub accounts: WithdrawAccounts<'a>,
    pub instruction_data: WithdrawInstructionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountInfo])> for Withdraw<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountInfo])) -> Result<Self, Self::Error> {
        let accounts = WithdrawAccounts::try_from(accounts)?;
        let instruction_data = WithdrawInstructionData::try_from(data)?;

        Ok(Self { accounts, instruction_data })
    }
}

impl<'a> Withdraw<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        let config = Config::load(&self.accounts.config)?; //or load_unchecked ?

        let accounts = &self.accounts;
        let data = &self.instruction_data;

        // （这个检测很重要） 验证用户已签名
        if !accounts.user.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 过期检查
        let clock = Clock::get()?;
        if clock.unix_timestamp > data.expiration {
            return Err(ProgramError::InvalidArgument);
        }

        //验证 AmmState 是否有效
        let state = config.state();
        if state != AmmState::Initialized as u8
            && state != AmmState::WithdrawOnly as u8
        {
            return Err(ProgramError::InvalidAccountData);
        }

        //todo 这个检查多余吗？ //太费性能了，改为和config里的mint_x和mint_y对比
        // //检查 vault_x 和 vault_y 的派生是否为关联代币账户（Associated Token Accounts）
        // let (vault_x, _) = find_program_address(
        //     &[
        //         self.accounts.config.key(),
        //         self.accounts.token_program.key(),
        //         config.mint_x(),
        //     ],
        //     &pinocchio_associated_token_account::ID.to_bytes(),
        // );
        // if vault_x.ne(self.accounts.vault_x.key()) {
        //     return Err(ProgramError::InvalidAccountData);
        // }

        // //check vault_y
        // let (vault_y, _) = find_program_address(
        //     &[
        //         self.accounts.config.key(),
        //         self.accounts.token_program.key(),
        //         config.mint_y(),
        //     ],
        //     &pinocchio_associated_token_account::ID.to_bytes(),
        // );
        // if vault_y.ne(self.accounts.vault_y.key()) {
        //     return Err(ProgramError::InvalidAccountData);
        // }

        // 反序列化代币信息
        let mint_lp = unsafe { Mint::from_account_info_unchecked(accounts.mint_lp)? };
        let vault_x = unsafe { TokenAccount::from_account_info_unchecked(accounts.vault_x)? };
        let vault_y = unsafe { TokenAccount::from_account_info_unchecked(accounts.vault_y)? };

        // 验证 vault 的 mint 与 config 一致，防止传入伪造 vault
        if vault_x.mint() != config.mint_x() || vault_y.mint() != config.mint_y() {
            return Err(ProgramError::InvalidAccountData);
        }

        //将金额从金库转移到用户的代币账户，并从用户的代币账户中销毁相应数量的 LP 代币
        //计算应退还的 X, Y 数量
        let (x, y) = if mint_lp.supply() == data.amount {
            // 全额提取：直接取走所有余额，防止舍入误差留下“尘埃”
            (vault_x.amount(), vault_y.amount())
        } else {
            let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
                vault_x.amount(),
                vault_y.amount(),
                mint_lp.supply(),
                data.amount,
                6, // LP decimals
            )
            .map_err(|_| ProgramError::ArithmeticOverflow)?;
            (amounts.x, amounts.y)
        };

        // 滑点检查
        if x < data.min_x || y < data.min_y {
            return Err(ProgramError::InvalidArgument);
        }

         // 销毁用户的 LP 代币 (用户签名)
         //todo 如果用户的lp代币在burn以后余额为0，那需要关闭用户对应的lp代币账户吗？现在不关的话，以后用户可以自己关闭吗？(不需要使用该crate的program了吧？)
         Burn {
            mint: accounts.mint_lp,
            account: accounts.user_lp_ata,
            authority: accounts.user,
            amount: data.amount,
        }
        .invoke()?;

        // 构造 Config PDA 签名以从金库转账
        let config_seeds = config.config_seeds();
        let signer = Signer::from(&config_seeds);

        // 转移 Token X 和 Y (Config PDA 签名)
        Transfer {
            from: accounts.vault_x,
            to: accounts.user_x_ata,
            authority: accounts.config,
            amount: x,
        }
        // .invoke_signed(&[signer.clone()])?;
        .invoke_signed(core::slice::from_ref(&signer))?;

        Transfer {
            from: accounts.vault_y,
            to: accounts.user_y_ata,
            authority: accounts.config,
            amount: y,
        }
        .invoke_signed(&[signer])?;


        Ok(())
    }
}

pub struct WithdrawAccounts<'a> {
    pub user: &'a AccountInfo,
    pub mint_lp: &'a AccountInfo,
    pub vault_x: &'a AccountInfo,
    pub vault_y: &'a AccountInfo,
    pub user_x_ata: &'a AccountInfo,
    pub user_y_ata: &'a AccountInfo,
    pub user_lp_ata: &'a AccountInfo,
    pub config: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for WithdrawAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            user,
            mint_lp,
            vault_x,
            vault_y,
            user_x_ata,
            user_y_ata,
            user_lp_ata,
            config,
            token_program,
            _,
        ] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        //todo need check ?

        Ok(Self {
            user,
            mint_lp,
            vault_x,
            vault_y,
            user_x_ata,
            user_y_ata,
            user_lp_ata,
            config,
            token_program,
        })
    }
}

pub struct WithdrawInstructionData {
    pub amount: u64,
    pub min_x: u64,
    pub min_y: u64,
    pub expiration: i64,//todo 为什么需要这个字段？
}

impl<'a> TryFrom<&'a [u8]> for WithdrawInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        const WITHDRAW_DATA_LEN: usize = size_of::<WithdrawInstructionData>();

        //len check
        if data.len() != WITHDRAW_DATA_LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let min_x = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let min_y = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let expiration = i64::from_le_bytes(data[24..32].try_into().unwrap());

        //todo 这些检查多余吗？
        //确保任何数量，例如 amount、max_y 和 max_x 都大于零，并且订单尚未过期，可以使用 Clock sysvar 进行检查。
        if amount == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        if min_x == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        if min_y == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self {
            amount,
            min_x,
            min_y,
            expiration,
        })
    }
}