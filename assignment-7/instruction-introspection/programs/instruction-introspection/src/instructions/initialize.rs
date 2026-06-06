use crate::{constants::*, error::AmmError, AmmConfig};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint as LpMint, Token},
    token_2022::spl_token_2022::{
        extension::{
            transfer_fee::MAX_FEE_BASIS_POINTS, BaseStateWithExtensions, ExtensionType,
            StateWithExtensions,
        },
        state::Mint as MintState,
    },
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        constraint = mint_a.key() < mint_b.key() @ AmmError::InvalidMintOrder,
        mint::token_program = token_program,
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
        mint::token_program = token_program,
    )]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = mint_a,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = mint_b,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_b: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        space = AmmConfig::DISCRIMINATOR.len() + AmmConfig::INIT_SPACE,
        seeds = [CONFIG_SEED, seed.to_le_bytes().as_ref()],
        bump
    )]
    pub config: Account<'info, AmmConfig>,

    #[account(
        init,
        payer = authority,
        seeds = [LP_SEED, config.key().as_ref()],
        bump,
        mint::decimals = 3,
        mint::authority = config,
        mint::token_program = token_program_lp,
    )]
    pub mint_lp: Account<'info, LpMint>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub token_program_lp: Program<'info, Token>,
}

impl<'info> Initialize<'info> {
    pub fn init(
        &mut self,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        require!(fee < MAX_FEE_BASIS_POINTS, AmmError::InvalidFee);

        self.config.set_inner(AmmConfig {
            seed,
            authority: authority.unwrap_or(self.authority.key()),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            mint_lp: self.mint_lp.key(),
            fee: fee as u32,
            locked: false,
            bump: bumps.config,
            lp_bump: bumps.mint_lp,
            reserve_a: 0,
            reserve_b: 0,
        });

        Ok(())
    }
}
