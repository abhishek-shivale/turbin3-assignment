pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("8qoU6VhEPekaod9EmVxU8BKND6SPfxhvWF2gaWhHAqgH");

#[program]
pub mod instruction_introspection {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
    ) -> Result<()> {
        ctx.accounts.init(seed, fee, authority, &ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, max_a: u64, max_b: u64) -> Result<()> {
        ctx.accounts.deposit(amount, max_a, max_b)
    }

    pub fn swap(ctx: Context<Swap>, side: Side, amount_in: u64, min_out: u64) -> Result<()> {
        ctx.accounts.swap(side, amount_in, min_out)
    }

    pub fn withdraw(ctx: Context<Withdraw>, min_a: u64, min_b: u64) -> Result<()> {
        ctx.accounts.withdraw(min_a, min_b)
    }
}
