use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, CloseAccount, Mint, Token, TokenAccount, Transfer},
};

use crate::constants::*;
use crate::error::MarketplaceError;
use crate::instructions::buy::mint_rewards;
use crate::state::{Marketplace, Offer};

#[derive(Accounts)]
pub struct MakeTokenOffer<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    pub marketplace: Account<'info, Marketplace>,

    #[account(address = marketplace.payment_mint)]
    pub payment_mint: Account<'info, Mint>,

    pub maker_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = bidder,
        space = 8 + Offer::INIT_SPACE,
        seeds = [OFFER_SEED, maker_mint.key().as_ref(), bidder.key().as_ref()],
        bump,
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        init,
        payer = bidder,
        associated_token::mint = payment_mint,
        associated_token::authority = offer,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = bidder,
    )]
    pub bidder_payment_ata: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> MakeTokenOffer<'info> {
    pub fn init_offer(&mut self, amount: u64, bumps: &MakeTokenOfferBumps) -> Result<()> {
        self.offer.set_inner(Offer {
            bidder: self.bidder.key(),
            mint: self.maker_mint.key(),
            amount,
            bump: bumps.offer,
        });
        Ok(())
    }

    pub fn deposit_tokens(&self, amount: u64) -> Result<()> {
        token::transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.bidder_payment_ata.to_account_info(),
                    to: self.vault.to_account_info(),
                    authority: self.bidder.to_account_info(),
                },
            ),
            amount,
        )
    }
}

#[derive(Accounts)]
pub struct AcceptTokenOffer<'info> {
    #[account(mut)]
    pub acceptor: Signer<'info>,

    #[account(mut)]
    pub bidder: SystemAccount<'info>,

    pub marketplace: Account<'info, Marketplace>,

    #[account(address = marketplace.payment_mint)]
    pub payment_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = acceptor,
        associated_token::mint = payment_mint,
        associated_token::authority = marketplace,
    )]
    pub treasury_token: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = acceptor,
        associated_token::mint = payment_mint,
        associated_token::authority = acceptor,
    )]
    pub acceptor_payment_ata: Account<'info, TokenAccount>,

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
        seeds = [REWARDS_SEED, marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
    )]
    pub rewards_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = acceptor,
        associated_token::mint = rewards_mint,
        associated_token::authority = acceptor,
    )]
    pub acceptor_rewards_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        close = bidder,
        seeds = [OFFER_SEED, maker_mint.key().as_ref(), bidder.key().as_ref()],
        bump = offer.bump,
        constraint = offer.bidder == bidder.key() @ MarketplaceError::Unauthorized,
        constraint = offer.mint == maker_mint.key() @ MarketplaceError::InvalidNft,
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = offer,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> AcceptTokenOffer<'info> {
    fn offer_signer_seeds(&self) -> [Vec<u8>; 4] {
        [
            OFFER_SEED.to_vec(),
            self.maker_mint.key().to_bytes().to_vec(),
            self.bidder.key().to_bytes().to_vec(),
            vec![self.offer.bump],
        ]
    }

    pub fn send_tokens(&self) -> Result<()> {
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

        let seeds = self.offer_signer_seeds();
        let signer_seeds: &[&[&[u8]]] = &[&[
            seeds[0].as_ref(),
            seeds[1].as_ref(),
            seeds[2].as_ref(),
            seeds[3].as_ref(),
        ]];

        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.vault.to_account_info(),
                    to: self.acceptor_payment_ata.to_account_info(),
                    authority: self.offer.to_account_info(),
                },
                signer_seeds,
            ),
            to_seller,
        )?;

        if fee > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.vault.to_account_info(),
                        to: self.treasury_token.to_account_info(),
                        authority: self.offer.to_account_info(),
                    },
                    signer_seeds,
                ),
                fee,
            )?;
        }
        Ok(())
    }

    pub fn close_vault(&self) -> Result<()> {
        let seeds = self.offer_signer_seeds();
        let signer_seeds: &[&[&[u8]]] = &[&[
            seeds[0].as_ref(),
            seeds[1].as_ref(),
            seeds[2].as_ref(),
            seeds[3].as_ref(),
        ]];

        token::close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                destination: self.bidder.to_account_info(),
                authority: self.offer.to_account_info(),
            },
            signer_seeds,
        ))
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

    pub fn mint_rewards(&self) -> Result<()> {
        mint_rewards(
            &self.token_program,
            &self.rewards_mint,
            &self.acceptor_rewards_ata,
            &self.marketplace,
        )
    }
}

#[derive(Accounts)]
pub struct CancelTokenOffer<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    pub payment_mint: Account<'info, Mint>,

    pub maker_mint: Account<'info, Mint>,

    #[account(
        mut,
        close = bidder,
        seeds = [OFFER_SEED, maker_mint.key().as_ref(), bidder.key().as_ref()],
        bump = offer.bump,
        constraint = offer.bidder == bidder.key() @ MarketplaceError::Unauthorized,
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = offer,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = bidder,
    )]
    pub bidder_payment_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> CancelTokenOffer<'info> {
    fn offer_signer_seeds(&self) -> [Vec<u8>; 4] {
        [
            OFFER_SEED.to_vec(),
            self.maker_mint.key().to_bytes().to_vec(),
            self.bidder.key().to_bytes().to_vec(),
            vec![self.offer.bump],
        ]
    }

    pub fn refund_tokens(&self) -> Result<()> {
        let seeds = self.offer_signer_seeds();
        let signer_seeds: &[&[&[u8]]] = &[&[
            seeds[0].as_ref(),
            seeds[1].as_ref(),
            seeds[2].as_ref(),
            seeds[3].as_ref(),
        ]];

        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.vault.to_account_info(),
                    to: self.bidder_payment_ata.to_account_info(),
                    authority: self.offer.to_account_info(),
                },
                signer_seeds,
            ),
            self.offer.amount,
        )
    }

    pub fn close_vault(&self) -> Result<()> {
        let seeds = self.offer_signer_seeds();
        let signer_seeds: &[&[&[u8]]] = &[&[
            seeds[0].as_ref(),
            seeds[1].as_ref(),
            seeds[2].as_ref(),
            seeds[3].as_ref(),
        ]];

        token::close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                destination: self.bidder.to_account_info(),
                authority: self.offer.to_account_info(),
            },
            signer_seeds,
        ))
    }
}
