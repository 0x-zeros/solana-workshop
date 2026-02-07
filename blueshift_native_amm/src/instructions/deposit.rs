use crate::state::Config;
use core::mem::size_of;
use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::Mint;
use pinocchio_token::{
    instructions::{CloseAccount, Transfer},
    state::TokenAccount,
};

use super::helpers::*;



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

        SignerAccount::check(user)?;
        TokenProgram::check(token_program)?;
        let config_data = Config::load(config)?;

        MintInterface::check_with_program(mint_lp, token_program)?;

        let (expected_mint_lp, _) = find_program_address(
            &[b"mint_lp", config.key().as_ref()],
            &crate::ID,
        );
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
