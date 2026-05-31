use anchor_lang::prelude::*;


#[constant]
pub const UPDATE_AUTHORITY: &[u8] = b"update_authority";

#[constant]
pub const STAKE_STATE: &[u8] = b"stake_state";

#[constant]
pub const REWARDS_MINT: &[u8] = b"rewards_mint";

#[constant]
pub const MINT_DECIMALS: u8 = 6;

#[constant]
pub const STAKED_KEY: &str = "staked";

#[constant]
pub const STAKED_AT: &str = "staked_at";

#[constant]
pub const LAST_CLAIMED_AT: &str = "last_claimed_at";

#[constant]
pub const STAKED_COUNT: &str = "staked_count";


