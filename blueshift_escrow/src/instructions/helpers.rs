use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::Mint;

/// 辅助结构体用于签名者账户检查
pub struct SignerAccount;

impl SignerAccount {
    /// 检查账户是否为签名者
    #[inline(always)]
    pub fn check(account: &AccountInfo) -> ProgramResult {
        if !account.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }
}

/// 辅助结构体用于 Program Account 操作
pub struct ProgramAccount;

impl ProgramAccount {
    /// 初始化一个 PDA 账户
    pub fn init<T>(
        payer: &AccountInfo,
        account: &AccountInfo,
        seeds: &[pinocchio::instruction::Seed],
        space: usize,
    ) -> ProgramResult {
        // 计算租金豁免所需的最小余额
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(space);

        // 创建账户
        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: space as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[pinocchio::instruction::Signer::from(seeds)])?;

        Ok(())
    }

    /// 检查账户是否由当前程序拥有
    #[inline(always)]
    pub fn check(account: &AccountInfo) -> ProgramResult {
        if !account.is_owned_by(&crate::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(())
    }

    /// 关闭 Program Account，将 lamports 转移到目标账户
    pub fn close(account: &AccountInfo, destination: &AccountInfo) -> ProgramResult {
        // 获取账户余额
        let dest_starting_lamports = destination.lamports();
        let account_lamports = account.lamports();

        //todo 这边是不是还是使用Transfer比较好？

        // 转移 lamports
        unsafe {
            *destination.borrow_mut_lamports_unchecked() = dest_starting_lamports
                .checked_add(account_lamports)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            *account.borrow_mut_lamports_unchecked() = 0;
        }

        // 清空账户数据
        account
            .try_borrow_mut_data()?
            .fill(0);

        Ok(())
    }
}

/// 辅助结构体用于 Mint 检查
pub struct MintInterface;

impl MintInterface {
    /// 检查账户是否为有效的 Token Mint
    #[inline(always)]
    pub fn check(account: &AccountInfo) -> ProgramResult {
        // 检查账户是否由 Token Program 或 Token-2022 Program 拥有
        if !account.is_owned_by(&pinocchio_token::ID) 
            && !account.is_owned_by(&SPL_TOKEN_2022_ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // 检查账户数据长度
        if account.data_len() != Mint::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }
}
//todo 这么hardcoding吗？
/// Token-2022 Program ID
const SPL_TOKEN_2022_ID: Pubkey = [
    0x06, 0xdd, 0xf6, 0xe1, 0xd7, 0x65, 0xa1, 0x93,
    0xd9, 0xcb, 0xe1, 0x46, 0xce, 0xeb, 0x79, 0xac,
    0x1c, 0xb4, 0x85, 0xed, 0x5f, 0x5b, 0x37, 0x91,
    0x3a, 0x8c, 0xf5, 0x85, 0x7e, 0xff, 0x00, 0xa9,
];

/// 辅助结构体用于 Associated Token Account 操作
pub struct AssociatedTokenAccount;

impl AssociatedTokenAccount {
    /// 创建 Associated Token Account
    pub fn init(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        // 验证 ATA 地址是否正确
        let ata_address = get_associated_token_address(owner.key(), mint.key());
        if account.key() != &ata_address {
            return Err(ProgramError::InvalidSeeds);
        }

        // 使用 pinocchio-associated-token-account crate
        // 由于我们使用的是老版本的 AccountInfo，这里我们手动调用 ATA program
        invoke_create_associated_token_account(
            payer,
            account,
            owner,
            mint,
            system_program,
            token_program,
        )
    }

    /// 创建 Associated Token Account（如果需要的话）
    pub fn init_if_needed(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        // 如果账户已经初始化，直接返回
        if account.lamports() > 0 {
            return Ok(());
        }

        // 否则创建账户
        Self::init(account, mint, payer, owner, system_program, token_program)
    }

    /// 检查 Associated Token Account 是否有效
    pub fn check(
        account: &AccountInfo,
        owner: &AccountInfo,
        mint: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        // 检查账户是否由 Token Program 拥有
        if account.owner() != token_program.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // 验证 ATA 地址是否正确
        let ata_address = get_associated_token_address(owner.key(), mint.key());
        if account.key() != &ata_address {
            return Err(ProgramError::InvalidSeeds);
        }

        Ok(())
    }
}

/// 计算 Associated Token Address
fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let seeds = &[
        wallet.as_ref(),
        pinocchio_token::ID.as_ref(),
        mint.as_ref(),
    ];
    
    let (address, _) = pinocchio::pubkey::find_program_address(
        seeds,
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    
    address
}

//todo 这么hardcoding吗？
/// Associated Token Program ID
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = [
    0x8c, 0x97, 0x25, 0x8f, 0x4e, 0x24, 0x89, 0xf1,
    0xbb, 0x3d, 0x10, 0x29, 0x14, 0x8e, 0x0d, 0x83,
    0x0b, 0x5a, 0x13, 0x99, 0xda, 0xff, 0x10, 0x84,
    0x04, 0x8e, 0x7b, 0xd8, 0xdb, 0xe9, 0xf8, 0x59,
];

/// 手动调用 Associated Token Account Program 创建 ATA
fn invoke_create_associated_token_account(
    payer: &AccountInfo,
    account: &AccountInfo,
    owner: &AccountInfo,
    mint: &AccountInfo,
    system_program: &AccountInfo,
    token_program: &AccountInfo,
) -> ProgramResult {
    use pinocchio::instruction::{AccountMeta, Instruction};
    
    let instruction = Instruction {
        program_id: &ASSOCIATED_TOKEN_PROGRAM_ID,
        accounts: &[
            AccountMeta::writable_signer(payer.key()),
            AccountMeta::writable(account.key()),
            AccountMeta::readonly(owner.key()),
            AccountMeta::readonly(mint.key()),
            AccountMeta::readonly(system_program.key()),
            AccountMeta::readonly(token_program.key()),
        ],
        data: &[0u8], // Create instruction discriminator
    };

    pinocchio::program::invoke(
        &instruction,
        &[payer, account, owner, mint, system_program, token_program],
    )
}
