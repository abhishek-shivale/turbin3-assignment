use anchor_lang::prelude::*;

#[constant]
pub const MARKETPLACE_SEED: &[u8] = b"marketplace";

#[constant]
pub const TREASURY_SEED: &[u8] = b"treasury";

#[constant]
pub const REWARDS_SEED: &[u8] = b"rewards";

#[constant]
pub const OFFER_SEED: &[u8] = b"offer";

pub const REWARDS_DECIMALS: u8 = 6;

pub const REWARD_AMOUNT: u64 = 1_000_000;

pub const BPS_DENOMINATOR: u64 = 10_000;

pub const MAX_NAME_LEN: usize = 32;
