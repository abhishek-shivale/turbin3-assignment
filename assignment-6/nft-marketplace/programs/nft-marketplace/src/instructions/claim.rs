use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer as sol_transfer, Transfer as SolTransfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::constants::*;
use crate::error::MarketplaceError;
use crate::state::Marketplace;

#[derive(Accounts)]
pub struct ClaimSolFee<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        has_one = admin @ MarketplaceError::Unauthorized,
    )]
    pub marketplace: Account<'info, Marketplace>,

    #[account(
        mut,
        seeds = [TREASURY_SEED, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> ClaimSolFee<'info> {
    pub fn claim_fees(&self, amount: u64) -> Result<()> {
        require!(
            self.treasury.lamports() >= amount,
            MarketplaceError::InsufficientFunds
        );

        let marketplace_key = self.marketplace.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            TREASURY_SEED,
            marketplace_key.as_ref(),
            &[self.marketplace.treasury_bump],
        ]];

        sol_transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                SolTransfer {
                    from: self.treasury.to_account_info(),
                    to: self.admin.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }
}

#[derive(Accounts)]
pub struct ClaimTokenFee<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        has_one = admin @ MarketplaceError::Unauthorized,
    )]
    pub marketplace: Account<'info, Marketplace>,

    #[account(address = marketplace.payment_mint)]
    pub payment_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = marketplace,
    )]
    pub treasury_token: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = payment_mint,
        associated_token::authority = admin,
    )]
    pub admin_payment_ata: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> ClaimTokenFee<'info> {
    pub fn claim_fees(&self, amount: u64) -> Result<()> {
        require!(
            self.treasury_token.amount >= amount,
            MarketplaceError::InsufficientFunds
        );

        let name = self.marketplace.name.clone();
        let signer_seeds: &[&[&[u8]]] =
            &[&[MARKETPLACE_SEED, name.as_bytes(), &[self.marketplace.bump]]];

        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.treasury_token.to_account_info(),
                    to: self.admin_payment_ata.to_account_info(),
                    authority: self.marketplace.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }
}
