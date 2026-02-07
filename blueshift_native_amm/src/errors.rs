use pinocchio::program_error::ProgramError;

#[repr(u32)]
pub enum AmmError {
    InvalidAmmState = 0,
    // 可按需增加更多，例如：
    // InvalidVault = 1,
    // InvalidLpMint = 2,
}

impl From<AmmError> for ProgramError {
    fn from(e: AmmError) -> Self {
        ProgramError::Custom(e as u32)
    }
}