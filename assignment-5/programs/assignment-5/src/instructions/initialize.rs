use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::Config;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
        seeds = [b"config", seed.to_le_bytes().as_ref()],
        bump
    )]
    pub config: Account<'info, Config>,

    pub mint_a: InterfaceAccount<'info, Mint>,

    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
            init,
            payer = owner,
            associated_token::mint = mint_a,
            associated_token::authority = config,
            associated_token::token_program = token_program)]
    pub vault_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
            init,
            payer = owner,
            associated_token::mint = mint_b,
            associated_token::authority = config,
            associated_token::token_program = token_program)]
    pub vault_b: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Initialize<'info> {
    pub fn init(&mut self, seed: u64, locked: bool, bump: u8) -> Result<()> {
        self.config.set_inner(Config {
            seed,
            owner: self.owner.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            locked,
            bump,
        });
        Ok(())
    }
}
