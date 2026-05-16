use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct VaultState {
    pub state_bump: u8,
    pub vault_bump: u8,
}
