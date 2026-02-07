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
    instructions::{CloseAccount, Transfer, MintTo},
    state::TokenAccount,
};

pub struct Deposit<'a> {
    pub accounts: DepositAccounts<'a>,
    pub instruction_data: DepositInstructionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountInfo])> for Deposit<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountInfo])) -> Result<Self, Self::Error> {
        let accounts = DepositAccounts::try_from(accounts)?;
        let instruction_data = DepositInstructionData::try_from(data)?;

        // Return the initialized struct
        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> Deposit<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        let config = Config::load(&self.accounts.config)?; //or load_unchecked ?

        let accounts = &self.accounts;
        let data = &self.instruction_data;

        //todo 这个检查多余吗？
        //check amm state
        if config.state() != AmmState::Initialized {
            return Err(AmmError::InvalidAmmState.into());
        }

        //todo 这个检查多余吗？
        //检查 vault_x 和 vault_y 的派生是否为关联代币账户（Associated Token Accounts）
        let (vault_x, _) = find_program_address(
            &[
                self.accounts.config.key(),
                self.accounts.token_program.key(),
                config.mint_x(),
            ],
            &pinocchio_associated_token_account::ID.to_bytes(),
        );
        if vault_x.ne(self.accounts.vault_x.key()) {
            return Err(ProgramError::InvalidAccountData);
        }

        //check vault_y
        let (vault_y, _) = find_program_address(
            &[
                self.accounts.config.key(),
                self.accounts.token_program.key(),
                config.mint_y(),
            ],
            &pinocchio_associated_token_account::ID.to_bytes(),
        );
        if vault_y.ne(self.accounts.vault_y.key()) {
            return Err(ProgramError::InvalidAccountData);
        }

        //计算存款数量
        // Deserialize the token accounts
        let mint_lp = unsafe { Mint::from_account_info_unchecked(self.accounts.mint_lp)? };
        let vault_x = unsafe { TokenAccount::from_account_info_unchecked(self.accounts.vault_x)? };
        let vault_y = unsafe { TokenAccount::from_account_info_unchecked(self.accounts.vault_y)? };


        // Grab the amounts to deposit
        let (x, y) = match mint_lp.supply() == 0 && vault_x.amount() == 0 && vault_y.amount() == 0 {
            //如果是首次存款，我们可以跳过 LP 代币和存款的计算，直接采用用户建议的数值
            true => (self.instruction_data.max_x, self.instruction_data.max_y),
            false => {
                let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                    vault_x.amount(),
                    vault_y.amount(),
                    mint_lp.supply(),
                    self.instruction_data.amount,
                    mint_lp.decimals() as u32,
                )
                .map_err(|_| ProgramError::InvalidArgument)?;

                (amounts.x, amounts.y)
            }
        };

        // Check for slippage
        if !(x <= self.instruction_data.max_x && y <= self.instruction_data.max_y) {
            return Err(ProgramError::InvalidArgument);
        }

        //todo 首次的LP 数量 怎么计算得来的？
        // 执行代币转移 (用户 -> 金库)
        Transfer {
            from: accounts.user_x_ata,
            to: accounts.vault_x,
            authority: accounts.user,
            amount: x,
        }
        .invoke()?;

        Transfer {
            from: accounts.user_y_ata,
            to: accounts.vault_y,
            authority: accounts.user,
            amount: y,
        }
        .invoke()?;

        //  签署并执行 MintTo (Config PDA -> 用户)
        let config_seeds = config.config_seeds();
        let signer = Signer::from(&config_seeds);

        MintTo {
            mint: accounts.mint_lp,
            account: accounts.user_lp_ata,
            mint_authority: accounts.config,
            amount: data.amount,
        }
        .invoke_signed(&[signer])?;

        Ok(())
    }
}

pub struct DepositAccounts<'a> {
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

impl<'a> TryFrom<&'a [AccountInfo]> for DepositAccounts<'a> {
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
        
        //todo 这些检查多余吗？
        SignerAccount::check(user)?;
        TokenProgram::check(token_program)?;
        let config_data = Config::load(config)?;

        MintInterface::check_with_program(mint_lp, token_program)?;

        let (expected_mint_lp, _) =
            find_program_address(&[b"mint_lp", config.key().as_ref()], &crate::ID);
        if mint_lp.key() != &expected_mint_lp {
            return Err(ProgramError::InvalidSeeds);
        }

        //todo check
        TokenAccountInterface::check_with_program(vault_x, token_program)?;
        TokenAccountInterface::check_with_program(vault_y, token_program)?;
        TokenAccountInterface::check_with_program(user_x_ata, token_program)?;
        TokenAccountInterface::check_with_program(user_y_ata, token_program)?;
        TokenAccountInterface::check_with_program(user_lp_ata, token_program)?;

        let vault_x_account = unsafe { TokenAccount::from_account_info_unchecked(vault_x)? };
        if vault_x_account.owner() != config.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if vault_x_account.mint() != config_data.mint_x() {
            return Err(ProgramError::InvalidAccountData);
        }

        let vault_y_account = unsafe { TokenAccount::from_account_info_unchecked(vault_y)? };
        if vault_y_account.owner() != config.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if vault_y_account.mint() != config_data.mint_y() {
            return Err(ProgramError::InvalidAccountData);
        }

        let user_x_account = unsafe { TokenAccount::from_account_info_unchecked(user_x_ata)? };
        if user_x_account.owner() != user.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if user_x_account.mint() != config_data.mint_x() {
            return Err(ProgramError::InvalidAccountData);
        }

        let user_y_account = unsafe { TokenAccount::from_account_info_unchecked(user_y_ata)? };
        if user_y_account.owner() != user.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if user_y_account.mint() != config_data.mint_y() {
            return Err(ProgramError::InvalidAccountData);
        }

        let user_lp_account = unsafe { TokenAccount::from_account_info_unchecked(user_lp_ata)? };
        if user_lp_account.owner() != user.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if user_lp_account.mint() != mint_lp.key() {
            return Err(ProgramError::InvalidAccountData);
        }

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

pub struct DepositInstructionData {
    pub amount: u64,
    pub max_x: u64,
    pub max_y: u64,
    pub expiration: i64,
}

impl<'a> TryFrom<&'a [u8]> for DepositInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        const DEPOSIT_DATA_LEN: usize = size_of::<DepositInstructionData>();

        //len check
        if data.len() != DEPOSIT_DATA_LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let max_x = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let max_y = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let expiration = i64::from_le_bytes(data[24..32].try_into().unwrap());

        //确保任何数量，例如 amount、max_y 和 max_x 都大于零，并且订单尚未过期，可以使用 Clock sysvar 进行检查。
        if amount == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        if max_x == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        if max_y == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        if expiration <= Clock::get()?.unix_timestamp {
            //todo 更有意思的error code
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self {
            amount,
            max_x,
            max_y,
            expiration,
        })
    }
}
