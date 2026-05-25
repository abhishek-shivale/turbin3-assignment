pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("6NvaFi7fwTf4nnK5sMJKRHgEKDyD5FZUTsE5cDb1ZQf2");

#[program]
pub mod assignment_5 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, seed: u64) -> Result<()> {
        ctx.accounts.init(seed, false, ctx.bumps.config)
    }
}
