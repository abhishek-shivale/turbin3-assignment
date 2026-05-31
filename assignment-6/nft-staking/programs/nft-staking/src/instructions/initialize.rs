use crate::state::StakeState;
use crate::{error::StakingError, MINT_DECIMALS, REWARDS_MINT};
use crate::{STAKE_STATE, UPDATE_AUTHORITY};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use mpl_core::accounts::BaseCollectionV1;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = StakeState::DISCRIMINATOR.len() + StakeState::INIT_SPACE,
        seeds = [STAKE_STATE, collection.key().as_ref()],
        bump
    )]
    pub stake_state: Box<Account<'info, StakeState>>,

    #[account(has_one = update_authority @ StakingError::InvalidUpdateAuthority)]
    pub collection: Box<Account<'info, BaseCollectionV1>>,

    #[account(seeds = [UPDATE_AUTHORITY, collection.key().as_ref()], bump)]
    pub update_authority: SystemAccount<'info>,

    #[account(
        init,
        payer = admin,
        mint::decimals = MINT_DECIMALS,
        mint::authority = stake_state,
        seeds = [REWARDS_MINT, stake_state.key().as_ref()],
        bump
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Initialize>, rewards_bps: u16, freeze_period: u16) -> Result<()> {
    ctx.accounts.stake_state.set_inner(StakeState {
        freeze_period,
        rewards_bps,
        reward_bump: ctx.bumps.rewards_mint,
        bump: ctx.bumps.stake_state,
    });
    Ok(())
}
