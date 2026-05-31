use anchor_lang::prelude::*;

use crate::constants::MAX_NAME_LEN;

#[account]
#[derive(InitSpace)]
pub struct Marketplace {
    pub admin: Pubkey,
    pub payment_mint: Pubkey,
    pub fee: u16,
    pub bump: u8,
    pub treasury_bump: u8,
    pub rewards_bump: u8,
    #[max_len(MAX_NAME_LEN)]
    pub name: String,
}

#[account]
#[derive(InitSpace)]
pub struct Listing {
    pub maker: Pubkey,
    pub mint: Pubkey,
    pub price: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub bidder: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub bump: u8,
}
