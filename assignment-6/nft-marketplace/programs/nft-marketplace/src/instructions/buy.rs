use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer as sol_transfer, Transfer as SolTransfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, CloseAccount, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::constants::*;
use crate::error::MarketplaceError;
use crate::state::{Listing, Marketplace};

fn split_price(price: u64, fee_bps: u16) -> Result<(u64, u64)> {
    let fee = (price as u128)
        .checked_mul(fee_bps as u128)
        .ok_or(MarketplaceError::Overflow)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(MarketplaceError::Overflow)? as u64;
    let to_seller = price.checked_sub(fee).ok_or(MarketplaceError::Overflow)?;
    Ok((to_seller, fee))
}

#[derive(Accounts)]
pub struct BuyWithSol<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    pub marketplace: Account<'info, Marketplace>,

    #[account(
        mut,
        seeds = [TREASURY_SEED, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    pub maker_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = maker_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_ata: Account<'info, TokenAccount>,

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

    #[account(
        mut,
        seeds = [REWARDS_SEED, marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
    )]
    pub rewards_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = rewards_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_rewards_ata: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> BuyWithSol<'info> {
    pub fn send_sol(&self) -> Result<()> {
        let (to_seller, fee) = split_price(self.listing.price, self.marketplace.fee)?;

        sol_transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                SolTransfer {
                    from: self.buyer.to_account_info(),
                    to: self.maker.to_account_info(),
                },
            ),
            to_seller,
        )?;

        if fee > 0 {
            sol_transfer(
                CpiContext::new(
                    self.system_program.to_account_info(),
                    SolTransfer {
                        from: self.buyer.to_account_info(),
                        to: self.treasury.to_account_info(),
                    },
                ),
                fee,
            )?;
        }
        Ok(())
    }

    pub fn receive_nft(&self) -> Result<()> {
        let marketplace_key = self.marketplace.key();
        let mint_key = self.maker_mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            marketplace_key.as_ref(),
            mint_key.as_ref(),
            &[self.listing.bump],
        ]];

        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.vault.to_account_info(),
                    to: self.buyer_ata.to_account_info(),
                    authority: self.listing.to_account_info(),
                },
                signer_seeds,
            ),
            1,
        )?;

        token::close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                destination: self.maker.to_account_info(),
                authority: self.listing.to_account_info(),
            },
            signer_seeds,
        ))
    }

    pub fn receive_rewards(&self) -> Result<()> {
        mint_rewards(
            &self.token_program,
            &self.rewards_mint,
            &self.buyer_rewards_ata,
            &self.marketplace,
        )
    }
}

#[derive(Accounts)]
pub struct BuyWithToken<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    pub marketplace: Account<'info, Marketplace>,

    #[account(address = marketplace.payment_mint)]
    pub payment_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_payment_ata: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = payment_mint,
        associated_token::authority = maker,
    )]
    pub maker_payment_ata: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = payment_mint,
        associated_token::authority = marketplace,
    )]
    pub treasury_token: Account<'info, TokenAccount>,

    pub maker_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = maker_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_ata: Account<'info, TokenAccount>,

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

    #[account(
        mut,
        seeds = [REWARDS_SEED, marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
    )]
    pub rewards_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = rewards_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_rewards_ata: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> BuyWithToken<'info> {
    pub fn send_tokens(&self) -> Result<()> {
        let (to_seller, fee) = split_price(self.listing.price, self.marketplace.fee)?;

        token::transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.buyer_payment_ata.to_account_info(),
                    to: self.maker_payment_ata.to_account_info(),
                    authority: self.buyer.to_account_info(),
                },
            ),
            to_seller,
        )?;

        if fee > 0 {
            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.buyer_payment_ata.to_account_info(),
                        to: self.treasury_token.to_account_info(),
                        authority: self.buyer.to_account_info(),
                    },
                ),
                fee,
            )?;
        }
        Ok(())
    }

    pub fn receive_nft(&self) -> Result<()> {
        let marketplace_key = self.marketplace.key();
        let mint_key = self.maker_mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            marketplace_key.as_ref(),
            mint_key.as_ref(),
            &[self.listing.bump],
        ]];

        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.vault.to_account_info(),
                    to: self.buyer_ata.to_account_info(),
                    authority: self.listing.to_account_info(),
                },
                signer_seeds,
            ),
            1,
        )?;

        token::close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                destination: self.maker.to_account_info(),
                authority: self.listing.to_account_info(),
            },
            signer_seeds,
        ))
    }

    pub fn receive_rewards(&self) -> Result<()> {
        mint_rewards(
            &self.token_program,
            &self.rewards_mint,
            &self.buyer_rewards_ata,
            &self.marketplace,
        )
    }
}

pub fn mint_rewards<'info>(
    token_program: &Program<'info, Token>,
    rewards_mint: &Account<'info, Mint>,
    recipient: &Account<'info, TokenAccount>,
    marketplace: &Account<'info, Marketplace>,
) -> Result<()> {
    let name = marketplace.name.clone();
    let signer_seeds: &[&[&[u8]]] = &[&[MARKETPLACE_SEED, name.as_bytes(), &[marketplace.bump]]];

    token::mint_to(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            MintTo {
                mint: rewards_mint.to_account_info(),
                to: recipient.to_account_info(),
                authority: marketplace.to_account_info(),
            },
            signer_seeds,
        ),
        REWARD_AMOUNT,
    )
}
