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

pub struct Initialize<'a> {
    pub accounts: InitializeAccounts<'a>,
    pub instruction_data: InitializeInstructionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountInfo])> for Initialize<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountInfo])) -> Result<Self, Self::Error> {
        let accounts = InitializeAccounts::try_from(accounts)?;
        let instruction_data: InitializeInstructionData =
            InitializeInstructionData::try_from(data)?;

        //Initialize the config account
        let seed_binding = instruction_data.seed.to_le_bytes();
        let config_seeds = [
            Seed::from(b"config"),
            Seed::from(&seed_binding),
            Seed::from(&instruction_data.mint_x),
            Seed::from(&instruction_data.mint_y),
            Seed::from(&instruction_data.config_bump),
        ];

        ProgramAccount::init::<Config>(
            accounts.initializer,
            accounts.config,
            &config_seeds[..],
            Config::LEN,
        )?;

        //mint_lp
        let mint_lp_seeds = [
            Seed::from(b"mint_lp"),
            Seed::from(accounts.config.key()),
            Seed::from(&instruction_data.lp_bump),
        ];

        MintInterface::init(
            accounts.initializer,
            accounts.mint_lp,
            &mint_lp_seeds[..],
            6, //hardcoded
            accounts.initializer,
            accounts.token_program,
        )?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> Initialize<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;

    pub fn process(&mut self) -> ProgramResult {
        //Populate the config account
        let mut data = self.accounts.config.try_borrow_mut_data()?;
        let config = Config::load_mut_unchecked(data.as_mut())?;

        config.set_inner(
            self.instruction_data.seed,
            self.instruction_data.authority,
            self.instruction_data.mint_x,
            self.instruction_data.mint_y,
            self.instruction_data.fee,
            self.instruction_data.config_bump,
        );

        Ok(())
    }
}

pub struct InitializeAccounts<'a> {
    pub initializer: &'a AccountInfo,
    pub mint_lp: &'a AccountInfo,
    pub config: &'a AccountInfo,

    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for InitializeAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            initializer,
            mint_lp,
            config,
            system_program,
            token_program,
            _,
        ] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // 随着经验的积累，您会注意到许多这些检查可以省略，而依赖于 CPI 本身强制执行的约束。
        //例如，对于此账户结构，不需要任何显式检查；如果不满足约束，程序将默认失败。

        //我们不需要对传入的账户进行显式检查。
        // 这是因为在实际操作中，如果有问题，指令会失败；要么在 CPI（跨程序调用）期间，要么通过我们在程序中设置的早期检查失败。
        // 例如，考虑 initializer 账户。我们期望它既是 signer 又是 mutable，但如果不是，CreateAccount 指令将会自动失败，因为它需要这些属性来满足 payer 的要求。
        // 同样地，如果传递的 config 账户具有无效的 mint_x 或 mint_y，任何尝试向协议中存入资金的操作都会在代币转移期间失败。
        // 随着经验的积累，您会发现可以省略许多检查，以保持指令的轻量化和优化，依赖系统和下游指令来强制执行约束

        Ok(Self {
            initializer,
            mint_lp,
            config,
            system_program,
            token_program,
        })
    }
}

#[repr(C, packed)]
pub struct InitializeInstructionData {
    pub seed: u64,
    pub fee: u16,
    pub mint_x: [u8; 32],
    pub mint_y: [u8; 32],
    pub config_bump: [u8; 1],
    pub lp_bump: [u8; 1],
    pub authority: [u8; 32],
}

impl TryFrom<&[u8]> for InitializeInstructionData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        const INITIALIZE_DATA_LEN_WITH_AUTHORITY: usize = size_of::<InitializeInstructionData>();
        const INITIALIZE_DATA_LEN: usize =
            INITIALIZE_DATA_LEN_WITH_AUTHORITY - size_of::<[u8; 32]>();

        match data.len() {
            INITIALIZE_DATA_LEN_WITH_AUTHORITY => {
                Ok(unsafe { (data.as_ptr() as *const Self).read_unaligned() })
            }
            INITIALIZE_DATA_LEN => {
                // If the authority is not present, we need to build the buffer and add it at the end before transmuting to the struct
                let mut raw: MaybeUninit<[u8; INITIALIZE_DATA_LEN_WITH_AUTHORITY]> =
                    MaybeUninit::uninit();
                let raw_ptr = raw.as_mut_ptr() as *mut u8;
                unsafe {
                    // Copy the provided data
                    core::ptr::copy_nonoverlapping(data.as_ptr(), raw_ptr, INITIALIZE_DATA_LEN);
                    // Add the authority to the end of the buffer
                    core::ptr::write_bytes(raw_ptr.add(INITIALIZE_DATA_LEN), 0, 32);
                    // Now transmute to the struct
                    Ok((raw.as_ptr() as *const Self).read_unaligned())
                }
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
