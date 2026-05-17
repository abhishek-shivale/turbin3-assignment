use crate::{error::EscrowError, state::Escrow};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    // key - maker pubkey
    // seed here we use because different seed different the escrow (single user, multi escrow support)
    #[account(init, payer = maker, space = Escrow::INIT_SPACE + Escrow::DISCRIMINATOR.len(), seeds = [b"escrow", maker.key().as_ref(), seed.to_le_bytes().as_ref()], bump )]
    pub escrow: Box<Account<'info, Escrow>>,

    // mint_a is mint account - a token defination
    // InterfaceAccount type support both token program in one program (token program + token 2022)
    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    // mint_b is mint account - for a token defination
    #[account(mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    // associated_token = actual place where in solana token is held
    // associated_token::mint = this ata must be for token a
    // associated_token::authority = this ata must owned by maker
    // associated_token::toke_program = token program of ata
    #[account(mut, associated_token::mint = mint_a, associated_token::authority = maker, associated_token::token_program = token_program)]
    pub maker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,

    // token lives here for lifetime of escrow program.
    #[account(init, payer=maker, associated_token::mint=mint_a, associated_token::authority=escrow, associated_token::token_program=token_program)]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // associated token program = solanas native program for ata's
    pub associated_token_program: Program<'info, AssociatedToken>,

    // token program = solana's native token program for token related activity
    pub token_program: Interface<'info, TokenInterface>,

    // system program = solana's native program use to create new account on (init)
    pub system_program: Program<'info, System>,
}

impl<'info> Make<'info> {
    fn populate_escrow(&mut self, seed: u64, receive: u64, bump: u8) -> Result<()> {
        // set_inner = set inner takes the Escrow struct and write it on chain
        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            receive: receive,
            bump,
        });
        Ok(())
    }

    fn deposit_token(&mut self, amount: u64) -> Result<()> {
        // transfer_checked = this is an anchor fn use for performing safe token transfer
        // CpiContext = cpi (cross program invocation) your escrow program calling the token program ::new create the context need to call program
        transfer_checked(
            CpiContext::new(
                self.token_program.key(),
                TransferChecked {
                    from: self.maker_ata_a.to_account_info(),
                    mint: self.mint_a.to_account_info(),
                    to: self.vault.to_account_info(),
                    authority: self.maker.to_account_info(),
                },
            ),
            amount,
            self.mint_a.decimals,
        );
        Ok(())
    }
}

pub fn handler(ctx: Context<Make>, seed: u64, receive: u64, amount: u64) -> Result<()> {
    require_gt!(receive, 0, EscrowError::InvalidAmount);
    require_gt!(amount, 0, EscrowError::InvalidAmount);

    ctx.accounts
        .populate_escrow(seed, receive, ctx.bumps.escrow)?;
    ctx.accounts.deposit_token(amount)?;
    Ok(())
}
