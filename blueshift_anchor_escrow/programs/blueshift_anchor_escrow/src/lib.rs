use anchor_lang::prelude::*;

declare_id!("FiXAcEKPeRALfM27oaQjM9S1CsWcrGz8L6kJS5wX37EP");

#[program]
pub mod blueshift_anchor_escrow {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
