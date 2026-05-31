use anchor_lang::prelude::*;
use mpl_core::ID as MPL_CORE_ID;

#[account]
#[derive(InitSpace)]
pub struct StakeState {
    pub rewards_bps: u16,
    pub freeze_period: u16,
    pub reward_bump: u8,
    pub bump: u8
}

#[derive(Clone)]
pub struct MplCore;

impl Id for MplCore {
    fn id() -> Pubkey {
        MPL_CORE_ID
    }
}
