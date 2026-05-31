use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer as sol_transfer, Transfer as SolTransfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::constants::*;
use crate::error::MarketplaceError;
use crate::state::{Marketplace, Offer};

#[derive(Accounts)]
pub struct MakeSolOffer<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    pub marketplace: Account<'info, Marketplace>,

    pub maker_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = bidder,
        space = 8 + Offer::INIT_SPACE,
        seeds = [OFFER_SEED, maker_mint.key().as_ref(), bidder.key().as_ref()],
        bump,
    )]
    pub offer: Account<'info, Offer>,

    pub system_program: Program<'info, System>,
}

impl<'info> MakeSolOffer<'info> {
    pub fn init_offer(&mut self, amount: u64, bumps: &MakeSolOfferBumps) -> Result<()> {
        self.offer.set_inner(Offer {
            bidder: self.bidder.key(),
            mint: self.maker_mint.key(),
            amount,
            bump: bumps.offer,
        });
        Ok(())
    }

    pub fn deposit_sol(&self, amount: u64) -> Result<()> {
        sol_transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                SolTransfer {
                    from: self.bidder.to_account_info(),
                    to: self.offer.to_account_info(),
                },
            ),
            amount,
        )
    }
}

#[derive(Accounts)]
pub struct AcceptSolOffer<'info> {
    #[account(mut)]
    pub acceptor: Signer<'info>,

    #[account(mut)]
    pub bidder: SystemAccount<'info>,

    pub marketplace: Account<'info, Marketplace>,

    #[account(
        mut,
        seeds = [TREASURY_SEED, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    pub maker_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = acceptor,
    )]
    pub acceptor_ata: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = acceptor,
        associated_token::mint = maker_mint,
        associated_token::authority = bidder,
    )]
    pub bidder_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        close = bidder,
        seeds = [OFFER_SEED, maker_mint.key().as_ref(), bidder.key().as_ref()],
        bump = offer.bump,
        constraint = offer.bidder == bidder.key() @ MarketplaceError::Unauthorized,
        constraint = offer.mint == maker_mint.key() @ MarketplaceError::InvalidNft,
    )]
    pub offer: Account<'info, Offer>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> AcceptSolOffer<'info> {
    pub fn send_sol(&self) -> Result<()> {
        let fee = (self.offer.amount as u128)
            .checked_mul(self.marketplace.fee as u128)
            .ok_or(MarketplaceError::Overflow)?
            .checked_div(BPS_DENOMINATOR as u128)
            .ok_or(MarketplaceError::Overflow)? as u64;
        let to_seller = self
            .offer
            .amount
            .checked_sub(fee)
            .ok_or(MarketplaceError::Overflow)?;

        let offer_ai = self.offer.to_account_info();
        **offer_ai.try_borrow_mut_lamports()? = offer_ai
            .lamports()
            .checked_sub(self.offer.amount)
            .ok_or(MarketplaceError::InsufficientFunds)?;

        let acceptor_ai = self.acceptor.to_account_info();
        **acceptor_ai.try_borrow_mut_lamports()? = acceptor_ai
            .lamports()
            .checked_add(to_seller)
            .ok_or(MarketplaceError::Overflow)?;

        if fee > 0 {
            let treasury_ai = self.treasury.to_account_info();
            **treasury_ai.try_borrow_mut_lamports()? = treasury_ai
                .lamports()
                .checked_add(fee)
                .ok_or(MarketplaceError::Overflow)?;
        }
        Ok(())
    }

    pub fn transfer_nft(&self) -> Result<()> {
        token::transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.acceptor_ata.to_account_info(),
                    to: self.bidder_ata.to_account_info(),
                    authority: self.acceptor.to_account_info(),
                },
            ),
            1,
        )
    }
}

#[derive(Accounts)]
pub struct CancelSolOffer<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    pub maker_mint: Account<'info, Mint>,

    #[account(
        mut,
        close = bidder,
        seeds = [OFFER_SEED, maker_mint.key().as_ref(), bidder.key().as_ref()],
        bump = offer.bump,
        constraint = offer.bidder == bidder.key() @ MarketplaceError::Unauthorized,
    )]
    pub offer: Account<'info, Offer>,

    pub system_program: Program<'info, System>,
}

impl<'info> CancelSolOffer<'info> {
    pub fn refund_sol(&self) -> Result<()> {
        Ok(())
    }
}
