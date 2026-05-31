use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, CloseAccount, Mint, Token, TokenAccount, Transfer},
};

use crate::error::MarketplaceError;
use crate::state::{Listing, Marketplace};

#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    pub marketplace: Account<'info, Marketplace>,

    #[account(
        constraint = maker_mint.decimals == 0 @ MarketplaceError::InvalidNft,
        constraint = maker_mint.supply == 1 @ MarketplaceError::InvalidNft,
    )]
    pub maker_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = maker,
    )]
    pub maker_ata: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = maker,
        space = 8 + Listing::INIT_SPACE,
        seeds = [marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        init,
        payer = maker,
        associated_token::mint = maker_mint,
        associated_token::authority = listing,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> List<'info> {
    pub fn create_listing(&mut self, price: u64, bumps: &ListBumps) -> Result<()> {
        self.listing.set_inner(Listing {
            maker: self.maker.key(),
            mint: self.maker_mint.key(),
            price,
            bump: bumps.listing,
        });

        let cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.maker_ata.to_account_info(),
                to: self.vault.to_account_info(),
                authority: self.maker.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, 1)
    }
}

#[derive(Accounts)]
pub struct Delist<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    pub marketplace: Account<'info, Marketplace>,

    pub maker_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = maker,
    )]
    pub maker_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        close = maker,
        seeds = [marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump = listing.bump,
        constraint = listing.maker == maker.key() @ MarketplaceError::Unauthorized,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = listing,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Delist<'info> {
    pub fn cancel_listing(&mut self) -> Result<()> {
        let marketplace_key = self.marketplace.key();
        let mint_key = self.maker_mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            marketplace_key.as_ref(),
            mint_key.as_ref(),
            &[self.listing.bump],
        ]];

        let transfer_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            Transfer {
                from: self.vault.to_account_info(),
                to: self.maker_ata.to_account_info(),
                authority: self.listing.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, 1)?;

        let close_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                destination: self.maker.to_account_info(),
                authority: self.listing.to_account_info(),
            },
            signer_seeds,
        );
        token::close_account(close_ctx)
    }
}
