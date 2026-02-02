use anchor_lang::prelude::*;

declare_id!("FPgL6qqt53QjRovkxAGeX8j8jh31dNvXABwt6CsCeRA3");

#[program]
pub mod blueshift_anchor_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
