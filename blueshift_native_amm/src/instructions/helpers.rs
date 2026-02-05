use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::{Mint, TokenAccount, InitializeMint2};

// ============================================================================
// Program ID 常量
// ============================================================================

// Associated Token Program ID
// Pubkey: ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = [
    0x8c, 0x97, 0x25, 0x8f, 0x4e, 0x24, 0x89, 0xf1,
    0xbb, 0x3d, 0x10, 0x29, 0x14, 0x8e, 0x0d, 0x83,
    0x0b, 0x5a, 0x13, 0x99, 0xda, 0xff, 0x10, 0x84,
    0x04, 0x8e, 0x7b, 0xd8, 0xdb, 0xe9, 0xf8, 0x59,
];

// Token-2022 Program ID
// Pubkey: TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
pub const SPL_TOKEN_2022_ID: Pubkey = [
    0x06, 0xdd, 0xf6, 0xe1, 0xd7, 0x65, 0xa1, 0x93,
    0xd9, 0xcb, 0xe1, 0x46, 0xce, 0xeb, 0x79, 0xac,
    0x1c, 0xb4, 0x85, 0xed, 0x5f, 0x5b, 0x37, 0x91,
    0x3a, 0x8c, 0xf5, 0x85, 0x7e, 0xff, 0x00, 0xa9,
];

// Memo Program ID
// Pubkey: MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr
pub const MEMO_PROGRAM_ID: Pubkey = [
    0x05, 0x4a, 0x53, 0x5a, 0x99, 0x29, 0x21, 0x06,
    0x4d, 0x24, 0xe8, 0x71, 0x60, 0xda, 0x38, 0x7c,
    0x7c, 0x35, 0xb5, 0xdd, 0xbc, 0x92, 0xbb, 0x81,
    0xe4, 0x1f, 0xa8, 0x40, 0x41, 0x05, 0x44, 0x8d,
];

// ============================================================================
// 签名者账户检查
// ============================================================================

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

    /// 检查账户是否为签名者且可写
    #[inline(always)]
    pub fn check_writable(account: &AccountInfo) -> ProgramResult {
        Self::check(account)?;
        if !account.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

// ============================================================================
// 系统账户检查
// ============================================================================

/// 辅助结构体用于系统账户检查
pub struct SystemAccount;

impl SystemAccount {
    /// 检查账户是否由系统程序拥有
    #[inline(always)]
    pub fn check(account: &AccountInfo) -> ProgramResult {
        if !account.is_owned_by(&pinocchio_system::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(())
    }

    /// 检查账户是否为系统程序本身
    #[inline(always)]
    pub fn check_program(account: &AccountInfo) -> ProgramResult {
        if account.key() != &pinocchio_system::ID {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(())
    }
}

// ============================================================================
// Program Account 操作
// ============================================================================

/// 辅助结构体用于 Program Account 操作
pub struct ProgramAccount;

impl ProgramAccount {
    /// 关闭账户时设置的 discriminator，防止重新初始化攻击
    pub const CLOSED_ACCOUNT_DISCRIMINATOR: u8 = 255;

    /// 初始化一个 PDA 账户
    pub fn init<T>(
        payer: &AccountInfo,
        account: &AccountInfo,
        seeds: &[pinocchio::instruction::Seed],
        space: usize,
    ) -> ProgramResult {
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(space);

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

    /// 初始化一个 PDA 账户，使用指定的 owner
    pub fn init_with_owner(
        payer: &AccountInfo,
        account: &AccountInfo,
        seeds: &[pinocchio::instruction::Seed],
        space: usize,
        owner: &Pubkey,
    ) -> ProgramResult {
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(space);

        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: space as u64,
            owner,
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

    /// 检查账户是否由当前程序拥有且可写
    #[inline(always)]
    pub fn check_writable(account: &AccountInfo) -> ProgramResult {
        Self::check(account)?;
        if !account.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    /// 检查账户是否由指定程序拥有
    #[inline(always)]
    pub fn check_owner(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
        if !account.is_owned_by(owner) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(())
    }

    /// 检查账户数据长度
    #[inline(always)]
    pub fn check_data_len(account: &AccountInfo, expected_len: usize) -> ProgramResult {
        if account.data_len() != expected_len {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    /// 检查账户是否未初始化（lamports == 0 且 data_len == 0）
    #[inline(always)]
    pub fn check_uninitialized(account: &AccountInfo) -> ProgramResult {
        if account.lamports() != 0 || account.data_len() != 0 {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        Ok(())
    }

    /// 检查账户是否已初始化
    #[inline(always)]
    pub fn check_initialized(account: &AccountInfo) -> ProgramResult {
        if account.lamports() == 0 {
            return Err(ProgramError::UninitializedAccount);
        }
        Ok(())
    }

    /// 检查账户是否未被关闭（第一个字节不是 CLOSED_ACCOUNT_DISCRIMINATOR）
    #[inline(always)]
    pub fn check_not_closed(account: &AccountInfo) -> ProgramResult {
        let data = account.try_borrow_data()?;
        if !data.is_empty() && data[0] == Self::CLOSED_ACCOUNT_DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    /// 关闭 Program Account，将 lamports 转移到目标账户
    /// 设置第一个字节为 255 防止账户被重新初始化
    pub fn close(account: &AccountInfo, destination: &AccountInfo) -> ProgramResult {
        let dest_starting_lamports = destination.lamports();
        let account_lamports = account.lamports();

        unsafe {
            *destination.borrow_mut_lamports_unchecked() = dest_starting_lamports
                .checked_add(account_lamports)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            *account.borrow_mut_lamports_unchecked() = 0;
        }

        let mut data = account.try_borrow_mut_data()?;
        if !data.is_empty() {
            data[0] = Self::CLOSED_ACCOUNT_DISCRIMINATOR;
            data[1..].fill(0);
        }

        Ok(())
    }

    /// 重新分配账户空间
    pub fn realloc(
        account: &AccountInfo,
        payer: &AccountInfo,
        new_size: usize,
    ) -> ProgramResult {
        let rent = Rent::get()?;
        let new_minimum_balance = rent.minimum_balance(new_size);
        let current_lamports = account.lamports();

        // 如果需要更多 lamports，从 payer 转移
        if new_minimum_balance > current_lamports {
            let lamports_diff = new_minimum_balance - current_lamports;
            let payer_lamports = payer.lamports();

            unsafe {
                *payer.borrow_mut_lamports_unchecked() = payer_lamports
                    .checked_sub(lamports_diff)
                    .ok_or(ProgramError::InsufficientFunds)?;
                *account.borrow_mut_lamports_unchecked() = new_minimum_balance;
            }
        }

        // 重新分配空间
        account.resize(new_size)?;

        Ok(())
    }
}

// ============================================================================
// Mint 检查
// ============================================================================

/// 辅助结构体用于 Mint 检查
pub struct MintInterface;

impl MintInterface {

    //todo 写得对不对
    /// 初始化一个 Mint 账户
    pub fn init<T>(
        payer: &AccountInfo,
        account: &AccountInfo,
        seeds: &[pinocchio::instruction::Seed],
        num_decimals: u8,
        mint_authority: &Pubkey,
        token_program: &Pubkey,
    ) -> ProgramResult {
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(Mint::LEN);

        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: space as u64,
            owner: token_program,
        }
        .invoke_signed(&[pinocchio::instruction::Signer::from(seeds)])?;

        InitializeMint2 {
            mint: account,
            decimals: num_decimals,
            authority: mint_authority,
        }
        .invoke_signed(&[pinocchio::instruction::Signer::from(seeds)])?;

        Ok(())
    }

    /// 检查账户是否为有效的 Token Mint
    /// Token Program: 精确匹配 Mint::LEN
    /// Token-2022: 允许 >= Mint::LEN（支持扩展数据）
    #[inline(always)]
    pub fn check(account: &AccountInfo) -> ProgramResult {
        let data_len = account.data_len();

        if account.is_owned_by(&pinocchio_token::ID) {
            if data_len != Mint::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
        } else if account.is_owned_by(&SPL_TOKEN_2022_ID) {
            if data_len < Mint::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
        } else {
            return Err(ProgramError::InvalidAccountOwner);
        }

        Ok(())
    }

    /// 检查账户是否为有效的 Token Mint，并验证 token program
    #[inline(always)]
    pub fn check_with_program(
        account: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        if account.owner() != token_program.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let data_len = account.data_len();
        if token_program.key() == &pinocchio_token::ID {
            if data_len != Mint::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
        } else if token_program.key() == &SPL_TOKEN_2022_ID {
            if data_len < Mint::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
        } else {
            return Err(ProgramError::IncorrectProgramId);
        }

        Ok(())
    }

    /// 获取 Mint 数据的只读引用
    #[inline(always)]
    pub fn get(account: &AccountInfo) -> Result<&Mint, ProgramError> {
        Self::check(account)?;
        // Safety: 我们已经验证了账户的 owner 和数据长度
        unsafe { Ok(Mint::from_account_info_unchecked(account)?) }
    }

    /// 检查 Mint 是否已初始化
    #[inline(always)]
    pub fn check_initialized(account: &AccountInfo) -> ProgramResult {
        let mint = Self::get(account)?;
        if !mint.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }
        Ok(())
    }

    /// 判断是否为 Token-2022 Mint
    #[inline(always)]
    pub fn is_token_2022(account: &AccountInfo) -> bool {
        account.is_owned_by(&SPL_TOKEN_2022_ID)
    }
}

// ============================================================================
// Token Account 检查
// ============================================================================

/// 辅助结构体用于 Token Account 检查
pub struct TokenAccountInterface;

impl TokenAccountInterface {
    /// 检查账户是否为有效的 Token Account
    #[inline(always)]
    pub fn check(account: &AccountInfo) -> ProgramResult {
        let data_len = account.data_len();

        if account.is_owned_by(&pinocchio_token::ID) {
            if data_len != TokenAccount::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
        } else if account.is_owned_by(&SPL_TOKEN_2022_ID) {
            if data_len < TokenAccount::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
        } else {
            return Err(ProgramError::InvalidAccountOwner);
        }

        Ok(())
    }

    /// 检查账户是否为有效的 Token Account，并验证 token program
    #[inline(always)]
    pub fn check_with_program(
        account: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        if account.owner() != token_program.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let data_len = account.data_len();
        if token_program.key() == &pinocchio_token::ID {
            if data_len != TokenAccount::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
        } else if token_program.key() == &SPL_TOKEN_2022_ID {
            if data_len < TokenAccount::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
        } else {
            return Err(ProgramError::IncorrectProgramId);
        }

        Ok(())
    }

    /// 获取 Token Account 数据的只读引用
    #[inline(always)]
    pub fn get(account: &AccountInfo) -> Result<&TokenAccount, ProgramError> {
        Self::check(account)?;
        unsafe { Ok(TokenAccount::from_account_info_unchecked(account)?) }
    }

    /// 检查 Token Account 的 owner
    #[inline(always)]
    pub fn check_owner(account: &AccountInfo, expected_owner: &Pubkey) -> ProgramResult {
        let token_account = Self::get(account)?;
        if token_account.owner() != expected_owner {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(())
    }

    /// 检查 Token Account 的 mint
    #[inline(always)]
    pub fn check_mint(account: &AccountInfo, expected_mint: &Pubkey) -> ProgramResult {
        let token_account = Self::get(account)?;
        if token_account.mint() != expected_mint {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    /// 检查 Token Account 的 owner 和 mint
    #[inline(always)]
    pub fn check_owner_and_mint(
        account: &AccountInfo,
        expected_owner: &Pubkey,
        expected_mint: &Pubkey,
    ) -> ProgramResult {
        let token_account = Self::get(account)?;
        if token_account.owner() != expected_owner {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if token_account.mint() != expected_mint {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    /// 检查 Token Account 是否有足够的余额
    #[inline(always)]
    pub fn check_balance(account: &AccountInfo, min_amount: u64) -> ProgramResult {
        let token_account = Self::get(account)?;
        if token_account.amount() < min_amount {
            return Err(ProgramError::InsufficientFunds);
        }
        Ok(())
    }

    /// 判断是否为 Token-2022 Account
    #[inline(always)]
    pub fn is_token_2022(account: &AccountInfo) -> bool {
        account.is_owned_by(&SPL_TOKEN_2022_ID)
    }
}

// ============================================================================
// Token Program 检查
// ============================================================================

/// 辅助结构体用于 Token Program 检查
pub struct TokenProgram;

impl TokenProgram {
    /// 检查账户是否为 Token Program 或 Token-2022 Program
    #[inline(always)]
    pub fn check(account: &AccountInfo) -> ProgramResult {
        if account.key() != &pinocchio_token::ID && account.key() != &SPL_TOKEN_2022_ID {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(())
    }

    /// 检查账户是否为标准 Token Program
    #[inline(always)]
    pub fn check_standard(account: &AccountInfo) -> ProgramResult {
        if account.key() != &pinocchio_token::ID {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(())
    }

    /// 检查账户是否为 Token-2022 Program
    #[inline(always)]
    pub fn check_2022(account: &AccountInfo) -> ProgramResult {
        if account.key() != &SPL_TOKEN_2022_ID {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(())
    }

    /// 判断是否为 Token-2022 Program
    #[inline(always)]
    pub fn is_token_2022(account: &AccountInfo) -> bool {
        account.key() == &SPL_TOKEN_2022_ID
    }
}

// ============================================================================
// Associated Token Account 操作
// ============================================================================

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
        let ata_address = get_associated_token_address(owner.key(), mint.key(), token_program.key());
        if account.key() != &ata_address {
            return Err(ProgramError::InvalidSeeds);
        }

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
        if account.lamports() > 0 {
            return Ok(());
        }

        Self::init(account, mint, payer, owner, system_program, token_program)
    }

    /// 创建 idempotent Associated Token Account（幂等操作，即使已存在也不会报错）
    pub fn init_idempotent(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        let ata_address = get_associated_token_address(owner.key(), mint.key(), token_program.key());
        if account.key() != &ata_address {
            return Err(ProgramError::InvalidSeeds);
        }

        invoke_create_associated_token_account_idempotent(
            payer,
            account,
            owner,
            mint,
            system_program,
            token_program,
        )
    }

    /// 检查 Associated Token Account 是否有效
    pub fn check(
        account: &AccountInfo,
        owner: &AccountInfo,
        mint: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        if account.owner() != token_program.key() {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let ata_address = get_associated_token_address(owner.key(), mint.key(), token_program.key());
        if account.key() != &ata_address {
            return Err(ProgramError::InvalidSeeds);
        }

        Ok(())
    }

    /// 检查 Associated Token Account 是否有效（更严格，验证 token account 数据）
    pub fn check_strict(
        account: &AccountInfo,
        owner: &AccountInfo,
        mint: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        Self::check(account, owner, mint, token_program)?;
        TokenAccountInterface::check_owner_and_mint(account, owner.key(), mint.key())?;
        Ok(())
    }
}

// ============================================================================
// PDA 辅助函数
// ============================================================================

/// 计算 Associated Token Address
/// 支持 Token Program 和 Token-2022
pub fn get_associated_token_address(
    wallet: &Pubkey,
    mint: &Pubkey,
    token_program_id: &Pubkey,
) -> Pubkey {
    let seeds = &[
        wallet.as_ref(),
        token_program_id.as_ref(),
        mint.as_ref(),
    ];

    let (address, _) = pinocchio::pubkey::find_program_address(
        seeds,
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    );

    address
}

/// 计算 Associated Token Address 并返回 bump
pub fn get_associated_token_address_and_bump(
    wallet: &Pubkey,
    mint: &Pubkey,
    token_program_id: &Pubkey,
) -> (Pubkey, u8) {
    let seeds = &[
        wallet.as_ref(),
        token_program_id.as_ref(),
        mint.as_ref(),
    ];

    pinocchio::pubkey::find_program_address(seeds, &ASSOCIATED_TOKEN_PROGRAM_ID)
}

/// 验证 PDA 地址
pub fn verify_pda(
    account: &AccountInfo,
    seeds: &[&[u8]],
    program_id: &Pubkey,
) -> Result<u8, ProgramError> {
    let (expected_address, bump) = pinocchio::pubkey::find_program_address(seeds, program_id);
    if account.key() != &expected_address {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(bump)
}

/// 使用已知的 bump 验证 PDA 地址（比 find_program_address 更高效）
pub fn verify_pda_with_bump(
    account: &AccountInfo,
    seeds: &[&[u8]],
    bump: u8,
    program_id: &Pubkey,
) -> ProgramResult {
    let mut seeds_with_bump: Vec<&[u8]> = seeds.to_vec();
    let bump_slice = &[bump];
    seeds_with_bump.push(bump_slice);

    let expected_address = pinocchio::pubkey::create_program_address(&seeds_with_bump, program_id)
        .map_err(|_| ProgramError::InvalidSeeds)?;

    if account.key() != &expected_address {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(())
}

// ============================================================================
// SOL 转账辅助函数
// ============================================================================

/// 转移 SOL（使用 CPI 调用 System Program）
pub fn transfer_sol(
    from: &AccountInfo,
    to: &AccountInfo,
    amount: u64,
) -> ProgramResult {
    pinocchio_system::instructions::Transfer {
        from,
        to,
        lamports: amount,
    }
    .invoke()
}

/// 转移 SOL（使用 PDA 签名）
pub fn transfer_sol_signed(
    from: &AccountInfo,
    to: &AccountInfo,
    amount: u64,
    seeds: &[pinocchio::instruction::Seed],
) -> ProgramResult {
    pinocchio_system::instructions::Transfer {
        from,
        to,
        lamports: amount,
    }
    .invoke_signed(&[pinocchio::instruction::Signer::from(seeds)])
}

/// 直接转移 lamports（不使用 CPI，更高效但需要注意安全性）
/// 仅用于当前程序拥有的账户之间的转账
pub fn transfer_lamports_unchecked(
    from: &AccountInfo,
    to: &AccountInfo,
    amount: u64,
) -> ProgramResult {
    let from_lamports = from.lamports();
    let to_lamports = to.lamports();

    unsafe {
        *from.borrow_mut_lamports_unchecked() = from_lamports
            .checked_sub(amount)
            .ok_or(ProgramError::InsufficientFunds)?;
        *to.borrow_mut_lamports_unchecked() = to_lamports
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    Ok(())
}

// ============================================================================
// Token 转账辅助函数
// ============================================================================

/// Token Transfer
pub fn transfer_tokens(
    from: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
) -> ProgramResult {
    pinocchio_token::instructions::Transfer {
        from,
        to,
        authority,
        amount,
    }
    .invoke()
}

/// Token Transfer（使用 PDA 签名）
pub fn transfer_tokens_signed(
    from: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    seeds: &[pinocchio::instruction::Seed],
) -> ProgramResult {
    pinocchio_token::instructions::Transfer {
        from,
        to,
        authority,
        amount,
    }
    .invoke_signed(&[pinocchio::instruction::Signer::from(seeds)])
}

/// Token Transfer Checked
pub fn transfer_tokens_checked(
    from: &AccountInfo,
    to: &AccountInfo,
    mint: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    decimals: u8,
) -> ProgramResult {
    pinocchio_token::instructions::TransferChecked {
        from,
        to,
        mint,
        authority,
        amount,
        decimals,
    }
    .invoke()
}

/// Token Transfer Checked（使用 PDA 签名）
pub fn transfer_tokens_checked_signed(
    from: &AccountInfo,
    to: &AccountInfo,
    mint: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    decimals: u8,
    seeds: &[pinocchio::instruction::Seed],
) -> ProgramResult {
    pinocchio_token::instructions::TransferChecked {
        from,
        to,
        mint,
        authority,
        amount,
        decimals,
    }
    .invoke_signed(&[pinocchio::instruction::Signer::from(seeds)])
}

// ============================================================================
// Token Mint/Burn 辅助函数
// ============================================================================

/// Mint Tokens
pub fn mint_tokens(
    mint: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
) -> ProgramResult {
    pinocchio_token::instructions::MintTo {
        mint,
        account: to,
        mint_authority: authority,
        amount,
    }
    .invoke()
}

/// Mint Tokens（使用 PDA 签名）
pub fn mint_tokens_signed(
    mint: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    seeds: &[pinocchio::instruction::Seed],
) -> ProgramResult {
    pinocchio_token::instructions::MintTo {
        mint,
        account: to,
        mint_authority: authority,
        amount,
    }
    .invoke_signed(&[pinocchio::instruction::Signer::from(seeds)])
}

/// Burn Tokens
pub fn burn_tokens(
    from: &AccountInfo,
    mint: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
) -> ProgramResult {
    pinocchio_token::instructions::Burn {
        account: from,
        mint,
        authority,
        amount,
    }
    .invoke()
}

/// Burn Tokens（使用 PDA 签名）
pub fn burn_tokens_signed(
    from: &AccountInfo,
    mint: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    seeds: &[pinocchio::instruction::Seed],
) -> ProgramResult {
    pinocchio_token::instructions::Burn {
        account: from,
        mint,
        authority,
        amount,
    }
    .invoke_signed(&[pinocchio::instruction::Signer::from(seeds)])
}

// ============================================================================
// Token Account 关闭辅助函数
// ============================================================================

/// 关闭 Token Account
pub fn close_token_account(
    token_account: &AccountInfo,
    destination: &AccountInfo,
    authority: &AccountInfo,
) -> ProgramResult {
    pinocchio_token::instructions::CloseAccount {
        account: token_account,
        destination,
        authority,
    }
    .invoke()
}

/// 关闭 Token Account（使用 PDA 签名）
pub fn close_token_account_signed(
    token_account: &AccountInfo,
    destination: &AccountInfo,
    authority: &AccountInfo,
    seeds: &[pinocchio::instruction::Seed],
) -> ProgramResult {
    pinocchio_token::instructions::CloseAccount {
        account: token_account,
        destination,
        authority,
    }
    .invoke_signed(&[pinocchio::instruction::Signer::from(seeds)])
}

// ============================================================================
// 内部辅助函数
// ============================================================================

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

/// 手动调用 Associated Token Account Program 创建 ATA（幂等版本）
fn invoke_create_associated_token_account_idempotent(
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
        data: &[1u8], // CreateIdempotent instruction discriminator
    };

    pinocchio::program::invoke(
        &instruction,
        &[payer, account, owner, mint, system_program, token_program],
    )
}

// ============================================================================
// 数学辅助函数
// ============================================================================

/// 安全乘法后除法，避免溢出
/// 计算 (a * b) / c，使用 u128 中间值
#[inline(always)]
pub fn mul_div(a: u64, b: u64, c: u64) -> Result<u64, ProgramError> {
    if c == 0 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    let result = (a as u128)
        .checked_mul(b as u128)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(c as u128)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if result > u64::MAX as u128 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    Ok(result as u64)
}

/// 安全乘法后除法，向上取整
#[inline(always)]
pub fn mul_div_ceil(a: u64, b: u64, c: u64) -> Result<u64, ProgramError> {
    if c == 0 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    let numerator = (a as u128)
        .checked_mul(b as u128)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let result = numerator
        .checked_add(c as u128 - 1)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(c as u128)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if result > u64::MAX as u128 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    Ok(result as u64)
}
