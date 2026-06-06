use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{error::AmmError, AmmConfig, Side, CONFIG_SEED};


#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        constraint = mint_a.key() < mint_b.key() @ AmmError::InvalidMintOrder,
        mint::token_program = token_program
    )]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mint::token_program = token_program
    )]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.bump,
    )]
    pub config: Box<Account<'info, AmmConfig>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_b: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}


impl<'info> Swap<'info> {
    pub fn swap(&mut self, side: Side, amount_in: u64, min_out: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount_in > 0, AmmError::InvalidAmount);

        let (reserve_in, reserve_out) = match side {
            Side::a => (self.vault_a.amount, self.vault_b.amount), // give A, get B
            Side::b => (self.vault_b.amount, self.vault_a.amount), // give B, get A
        };
        require!(reserve_in > 0 && reserve_out > 0, AmmError::InvalidAmount);

        let fee = self.config.fee as u128; // basis points (out of 10_000)
        let amount_in_after_fee = (amount_in as u128)
            .checked_mul(10_000u128.checked_sub(fee).ok_or(AmmError::InvalidFee)?)
            .and_then(|n| n.checked_div(10_000))
            .ok_or(AmmError::InvalidAmount)?;

        let new_reserve_in = (reserve_in as u128)
            .checked_add(amount_in_after_fee)
            .ok_or(AmmError::InvalidAmount)?;
        let k = (reserve_in as u128)
            .checked_mul(reserve_out as u128)
            .ok_or(AmmError::InvalidAmount)?;
        let amount_out = (reserve_out as u128)
            .checked_sub(k.checked_div(new_reserve_in).ok_or(AmmError::InvalidAmount)?)
            .ok_or(AmmError::InvalidAmount)? as u64;

        require!(amount_out > 0, AmmError::InvalidAmount);
        require!(amount_out >= min_out, AmmError::SlippageExceeded);

        self.transfer_in(&side, amount_in)?;
        self.transfer_out(&side, amount_out)?;

        Ok(())
    }

    fn transfer_in(&self, side: &Side, amount: u64) -> Result<()> {
        let (from, mint, to, decimals) = match side {
            Side::a => (
                self.user_a.to_account_info(),
                self.mint_a.to_account_info(),
                self.vault_a.to_account_info(),
                self.mint_a.decimals,
            ),
            Side::b => (
                self.user_b.to_account_info(),
                self.mint_b.to_account_info(),
                self.vault_b.to_account_info(),
                self.mint_b.decimals,
            ),
        };

        let cpi_accounts = TransferChecked {
            from,
            mint,
            to,
            authority: self.user.to_account_info(),
        };
        transfer_checked(
            CpiContext::new(self.token_program.key(), cpi_accounts),
            amount,
            decimals,
        )
    }

    fn transfer_out(&self, side: &Side, amount: u64) -> Result<()> {
        let (from, mint, to, decimals) = match side {
            Side::a => (
                self.vault_b.to_account_info(),
                self.mint_b.to_account_info(),
                self.user_b.to_account_info(),
                self.mint_b.decimals,
            ),
            Side::b => (
                self.vault_a.to_account_info(),
                self.mint_a.to_account_info(),
                self.user_a.to_account_info(),
                self.mint_a.decimals,
            ),
        };

        let seed_bytes = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] =
            &[&[CONFIG_SEED, seed_bytes.as_ref(), &[self.config.bump]]];

        let cpi_accounts = TransferChecked {
            from,
            mint,
            to,
            authority: self.config.to_account_info(),
        };
        transfer_checked(
            CpiContext::new_with_signer(self.token_program.key(), cpi_accounts, signer_seeds),
            amount,
            decimals,
        )
    }
}