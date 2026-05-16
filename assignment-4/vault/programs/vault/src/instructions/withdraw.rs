use crate::state::VaultState;
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};


#[derive(Accounts)]
pub struct Withdraw<'info>{
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(seeds = [b"state", vault_state.key().as_ref()], bump=vault_state.state_bump)]
    pub vault_state: Account<'info, VaultState>,
    #[account(mut, seeds = [b"vault", vault_state.key().as_ref() ], bump=vault_state.vault_bump)]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}


impl <'info> Withdraw <'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        let cpi_account = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info()
        };

        let key = self.vault_state.to_account_info().key();
        let seeds = &[b"vault", key.as_ref(), &[self.vault_state.vault_bump]];

        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(System::id(), cpi_account, signer);

        transfer(cpi_ctx, amount)?;

       Ok(()) 
    }
}


