use anchor_lang::prelude::*;

use crate::VaultState;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(init, seeds = [b"state", vault_state.key().as_ref()],payer=user, bump, space = 8 + VaultState::INIT_SPACE)]
    pub vault_state: Account<'info, VaultState>,

    #[account(seeds = [b"vault", vault_state.key().as_ref() ], bump)]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl <'info> Initialize <'info> {
   pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
       self.vault_state.vault_bump = bumps.vault;
       self.vault_state.state_bump = bumps.vault_state;

       Ok(())
   } 
}
