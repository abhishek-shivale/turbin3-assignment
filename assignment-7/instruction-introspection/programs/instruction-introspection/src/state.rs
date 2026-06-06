use anchor_lang::prelude::*;
use crate::error::AmmError;

#[account]
#[derive(InitSpace)]
pub struct AmmConfig{
    pub seed: u64,
    pub authority: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub mint_lp: Pubkey,
    pub fee: u32,
    pub locked: bool,
    pub bump: u8,
    pub lp_bump: u8,
    pub reserve_a: u64,
    pub reserve_b: u64,
}

impl AmmConfig{
    pub fn require_authority(&self, signer: &Pubkey) -> Result<()> {
        require_keys_eq!(self.authority, *signer, AmmError::UnAuthorized);
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum Side {
    a,
    b
}