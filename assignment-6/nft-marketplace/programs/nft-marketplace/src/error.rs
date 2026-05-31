use anchor_lang::prelude::*;

#[error_code]
pub enum MarketplaceError {
    #[msg("Name must be between 1 and 32 characters")]
    InvalidName,
    #[msg("Fee basis points cannot exceed 10000")]
    FeeTooHigh,
    #[msg("Only the marketplace admin may perform this action")]
    Unauthorized,
    #[msg("Insufficient funds in treasury")]
    InsufficientFunds,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Provided mint is not a valid NFT")]
    InvalidNft,
}
