use anchor_lang::prelude::*;
use anchor_spl::{
    token::Mint as LpMint,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use solana_instructions_sysvar::{
    load_current_index_checked, load_instruction_at_checked, ID as INSTRUCTIONS_SYSVAR_ID,
};

use crate::{constants::*, error::AmmError, AmmConfig, Side};

const BURN_DISCRIMINATOR: u8 = 8;
const BURN_CHECKED_DISCRIMINATOR: u8 = 15;

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

    /// the instructions-sysvar loaders for introspection.
    #[account(address = INSTRUCTIONS_SYSVAR_ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, min_a: u64, min_b: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);

        let burned = self.verify_prior_burn()?;
        require!(burned > 0, AmmError::InvalidAmount);

        let supply_before = (self.mint_lp.supply as u128)
            .checked_add(burned as u128)
            .ok_or(AmmError::InvalidAmount)?;
        require!(supply_before > 0, AmmError::InvalidAmount);

        let amount_a = (burned as u128)
            .checked_mul(self.config.reserve_a as u128)
            .and_then(|n| n.checked_div(supply_before))
            .ok_or(AmmError::InvalidAmount)? as u64;
        let amount_b = (burned as u128)
            .checked_mul(self.config.reserve_b as u128)
            .and_then(|n| n.checked_div(supply_before))
            .ok_or(AmmError::InvalidAmount)? as u64;

        require!(amount_a >= min_a && amount_b >= min_b, AmmError::SlippageExceeded);

        self.config.reserve_a = self
            .config
            .reserve_a
            .checked_sub(amount_a)
            .ok_or(AmmError::InvalidAmount)?;
        self.config.reserve_b = self
            .config
            .reserve_b
            .checked_sub(amount_b)
            .ok_or(AmmError::InvalidAmount)?;

        self.transfer_out(Side::a, amount_a)?;
        self.transfer_out(Side::b, amount_b)
    }

    fn verify_prior_burn(&self) -> Result<u64> {
        let ixs = self.instructions_sysvar.to_account_info();

        let current = load_current_index_checked(&ixs)
            .map_err(|_| error!(AmmError::MissingBurnInstruction))? as usize;
        require!(current > 0, AmmError::MissingBurnInstruction);

        let prev = load_instruction_at_checked(current - 1, &ixs)
            .map_err(|_| error!(AmmError::MissingBurnInstruction))?;

        require_keys_eq!(
            prev.program_id,
            anchor_spl::token::ID,
            AmmError::InvalidBurnProgram
        );

        require!(prev.data.len() >= 9, AmmError::InvalidBurnIxData);
        require!(
            matches!(prev.data[0], BURN_DISCRIMINATOR | BURN_CHECKED_DISCRIMINATOR),
            AmmError::InvalidBurnIxData
        );
        let burned = u64::from_le_bytes(
            prev.data[1..9]
                .try_into()
                .map_err(|_| error!(AmmError::InvalidBurnIxData))?,
        );

        require!(prev.accounts.len() >= 3, AmmError::InvalidBurnIxData);
        require_keys_eq!(
            prev.accounts[1].pubkey,
            self.mint_lp.key(),
            AmmError::BurnMintMismatch
        );
        require_keys_eq!(
            prev.accounts[2].pubkey,
            self.user.key(),
            AmmError::BurnOwnerMismatch
        );
        require!(prev.accounts[2].is_signer, AmmError::BurnOwnerMismatch);

        Ok(burned)
    }

    fn transfer_out(&self, side: Side, amount: u64) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }

        let (from, mint, to, decimals) = match side {
            Side::a => (
                self.vault_a.to_account_info(),
                self.mint_a.to_account_info(),
                self.user_a.to_account_info(),
                self.mint_a.decimals,
            ),
            Side::b => (
                self.vault_b.to_account_info(),
                self.mint_b.to_account_info(),
                self.user_b.to_account_info(),
                self.mint_b.decimals,
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
