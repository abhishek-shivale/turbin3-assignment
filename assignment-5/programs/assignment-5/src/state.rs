use anchor_lang::prelude::*;



#[derive(InitSpace)]
#[account]
pub struct Config {
    pub seed: u64,
    pub owner: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub locked: bool,
    pub bump: u8
}
