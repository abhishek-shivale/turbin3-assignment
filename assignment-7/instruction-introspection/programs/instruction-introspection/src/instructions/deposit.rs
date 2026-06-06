use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, Mint as LpMint, MintTo, Token, TokenAccount as LpTokenAccount},
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{constants::*, error::AmmError, AmmConfig};

#[derive(Accounts)]
pub struct Deposit<'info> {
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
        has_one = mint_lp,
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.bump,
    )]
    pub config: Box<Account<'info, AmmConfig>>,

    #[account(
        mut,
        seeds = [LP_SEED, config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub mint_lp: Box<Account<'info, LpMint>>,

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

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
        associated_token::token_program = token_program_lp,
    )]
    pub user_lp: Box<Account<'info, LpTokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,

    pub token_program_lp: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64, max_a: u64, max_b: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);

        let lp_supply = self.mint_lp.supply;

        let (x, y) = if lp_supply == 0 {
            require!(max_a > 0 && max_b > 0, AmmError::InvalidAmount);
            (max_a, max_b)
        } else {
            let x = (amount as u128)
                .checked_mul(self.vault_a.amount as u128)
                .and_then(|n| n.checked_div(lp_supply as u128))
                .ok_or(AmmError::InvalidAmount)? as u64;
            let y = (amount as u128)
                .checked_mul(self.vault_b.amount as u128)
                .and_then(|n| n.checked_div(lp_supply as u128))
                .ok_or(AmmError::InvalidAmount)? as u64;
            (x, y)
        };

        require!(x <= max_a && y <= max_b, AmmError::SlippageExceeded);

        self.transfer_to_vault(true, x)?;
        self.transfer_to_vault(false, y)?;

        let seed_bytes = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] =
            &[&[CONFIG_SEED, seed_bytes.as_ref(), &[self.config.bump]]];

        let cpi_accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.user_lp.to_account_info(),
            authority: self.config.to_account_info(),
        };
        mint_to(
            CpiContext::new_with_signer(
                self.token_program_lp.key(),
                cpi_accounts,
                signer_seeds,
            ),
            amount,
        )?;

        self.config.reserve_a = self.vault_a.amount.checked_add(x).ok_or(AmmError::InvalidAmount)?;
        self.config.reserve_b = self.vault_b.amount.checked_add(y).ok_or(AmmError::InvalidAmount)?;

        Ok(())
    }

    fn transfer_to_vault(&self, is_a: bool, amount: u64) -> Result<()> {
        let (from, mint, to, decimals) = if is_a {
            (
                self.user_a.to_account_info(),
                self.mint_a.to_account_info(),
                self.vault_a.to_account_info(),
                self.mint_a.decimals,
            )
        } else {
            (
                self.user_b.to_account_info(),
                self.mint_b.to_account_info(),
                self.vault_b.to_account_info(),
                self.mint_b.decimals,
            )
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
}
