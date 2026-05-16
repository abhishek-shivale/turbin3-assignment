use crate::state::VaultState;
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};


#[derive(Accounts)]
pub struct Deposit<'info>{
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(seeds = [b"state", vault_state.key().as_ref()], bump=vault_state.state_bump)]
    pub vault_state: Account<'info, VaultState>,
    #[account(seeds = [b"vault", vault_state.key().as_ref() ], bump=vault_state.vault_bump)]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}


impl <'info> Deposit <'info> {
    pub fn deposit (&mut self, amount: u64) -> Result<()> {
        let cpi_account = Transfer {
            to: self.vault.to_account_info(),
            from: self.user.to_account_info()
        };

        let cpi_ctx = CpiContext::new(System::id(), cpi_account);

        transfer(cpi_ctx, amount)?;

       Ok(()) 
    }
}

