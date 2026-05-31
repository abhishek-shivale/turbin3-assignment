use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

use crate::constants::*;
use crate::error::MarketplaceError;
use crate::state::Marketplace;

#[derive(Accounts)]
#[instruction(name: String)]
pub struct Init<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Marketplace::INIT_SPACE,
        seeds = [MARKETPLACE_SEED, name.as_bytes()],
        bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    #[account(
        seeds = [TREASURY_SEED, marketplace.key().as_ref()],
        bump,
    )]
    pub treasury: SystemAccount<'info>,

    pub payment_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        seeds = [REWARDS_SEED, marketplace.key().as_ref()],
        bump,
        mint::decimals = REWARDS_DECIMALS,
        mint::authority = marketplace,
    )]
    pub rewards_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Init<'info> {
    pub fn init(&mut self, name: String, fee: u16, bumps: &InitBumps) -> Result<()> {
        require!(
            !name.is_empty() && name.len() <= MAX_NAME_LEN,
            MarketplaceError::InvalidName
        );
        require!(fee <= BPS_DENOMINATOR as u16, MarketplaceError::FeeTooHigh);

        self.marketplace.set_inner(Marketplace {
            admin: self.admin.key(),
            payment_mint: self.payment_mint.key(),
            fee,
            bump: bumps.marketplace,
            treasury_bump: bumps.treasury,
            rewards_bump: bumps.rewards_mint,
            name,
        });
        Ok(())
    }
}
