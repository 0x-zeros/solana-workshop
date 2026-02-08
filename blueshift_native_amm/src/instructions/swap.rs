use crate::state::{AmmState, Config};
use constant_product_curve::{ConstantProduct, LiquidityPair};
use core::mem::size_of;
use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::Signer,
    program_error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};
use pinocchio_token::{
    instructions::Transfer,
    state::TokenAccount,
};

pub struct Swap<'a> {
    pub accounts: SwapAccounts<'a>,
    pub instruction_data: SwapInstructionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountInfo])> for Swap<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountInfo])) -> Result<Self, Self::Error> {
        let accounts = SwapAccounts::try_from(accounts)?;
        let instruction_data = SwapInstructionData::try_from(data)?;

        // Return the initialized struct
        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}
impl<'a> Swap<'a> {
    pub const DISCRIMINATOR: &'a u8 = &3;

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
        if state != AmmState::Initialized as u8 {
            return Err(ProgramError::InvalidAccountData);
        }

        //反序列化代币信息
        let vault_x = unsafe { TokenAccount::from_account_info_unchecked(accounts.vault_x)? };
        let vault_y = unsafe { TokenAccount::from_account_info_unchecked(accounts.vault_y)? };
        let user_x_ata = unsafe { TokenAccount::from_account_info_unchecked(accounts.user_x_ata)? };
        let user_y_ata = unsafe { TokenAccount::from_account_info_unchecked(accounts.user_y_ata)? };

        //验证 vault 的 mint 与 config 一致，防止传入伪造 vault
        if vault_x.mint() != config.mint_x() || vault_y.mint() != config.mint_y() {
            return Err(ProgramError::InvalidAccountData);
        }

        //todo 这个检查多余吗？
        //验证 user_x_ata 和 user_y_ata 的 mint 与 config 一致，防止传入伪造 user_x_ata 和 user_y_ata
        if user_x_ata.mint() != config.mint_x() || user_y_ata.mint() != config.mint_y() {
            return Err(ProgramError::InvalidAccountData);
        }

        // Swap Calculations
        let mut curve = ConstantProduct::init(
            vault_x.amount(),
            vault_y.amount(),
            vault_x.amount(),
            config.fee(),
            None,
        )
        .map_err(|_| ProgramError::Custom(1))?;
        let p = match data.is_x {
            true => LiquidityPair::X,
            false => LiquidityPair::Y,
        };
        let swap_result = curve
            .swap(p, self.instruction_data.amount, self.instruction_data.min)
            .map_err(|_| ProgramError::Custom(1))?;
        // Check for correct values
        if swap_result.deposit == 0 || swap_result.withdraw == 0 {
            return Err(ProgramError::InvalidArgument);
        }

        //转账逻辑. 检查is_x值，并将from金额转入金库，将to金额转入用户的代币账户
        // 构造 Config PDA 签名以从金库转账
        let config_seeds = config.config_seeds();
        let signer = Signer::from(&config_seeds);
        if data.is_x {
            //x to y
            Transfer {
                from: accounts.user_x_ata,
                to: accounts.vault_x,
                authority: accounts.user,
                amount: swap_result.deposit,
            }
            .invoke()?;
            Transfer {
                from: accounts.vault_y,
                to: accounts.user_y_ata,
                authority: accounts.config,
                amount: swap_result.withdraw,
            }
            .invoke_signed(&[signer])?;
        } else {
            //y to x
            Transfer {
                from: accounts.user_y_ata,
                to: accounts.vault_y,
                authority: accounts.user,
                amount: swap_result.deposit,
            }
            .invoke()?;
            Transfer {
                from: accounts.vault_x,
                to: accounts.user_x_ata,
                authority: accounts.config,
                amount: swap_result.withdraw,
            }
            .invoke_signed(&[signer])?;
        }

        Ok(())
    }
}

pub struct SwapAccounts<'a> {
    pub user: &'a AccountInfo,
    pub user_x_ata: &'a AccountInfo,
    pub user_y_ata: &'a AccountInfo,
    pub vault_x: &'a AccountInfo,
    pub vault_y: &'a AccountInfo,
    pub config: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for SwapAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            user,
            user_x_ata,
            user_y_ata,
            vault_x,
            vault_y,
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
            user_x_ata,
            user_y_ata,
            vault_x,
            vault_y,
            config,
            token_program,
        })
    }
}

pub struct SwapInstructionData {
    pub is_x: bool,
    pub amount: u64,
    pub min: u64,
    pub expiration: i64,
}

impl<'a> TryFrom<&'a [u8]> for SwapInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        const SWAP_DATA_LEN: usize = size_of::<SwapInstructionData>();

        //len check
        if data.len() != SWAP_DATA_LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        let is_x = data[0] == 1;
        let amount = u64::from_le_bytes(data[1..9].try_into().unwrap());
        let min = u64::from_le_bytes(data[9..17].try_into().unwrap());
        let expiration = i64::from_le_bytes(data[17..25].try_into().unwrap());

        //todo check ?
        if amount == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        if min == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self {
            is_x,
            amount,
            min,
            expiration,
        })
    }
}
