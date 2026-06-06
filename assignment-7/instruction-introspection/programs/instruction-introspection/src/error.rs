use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Signer is not authorized for this action.")]
    UnAuthorized,
    #[msg("Invalid mint order.")]
    InvalidMintOrder,
    #[msg("Pool is locked.")]
    PoolLocked,
    #[msg("Invalid fee")]
    InvalidFee,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Slippage exceeded")]
    SlippageExceeded,
    #[msg("Missing prior burn instruction")]
    MissingBurnInstruction,
    #[msg("Prior instruction is not the SPL Token program")]
    InvalidBurnProgram,
    #[msg("Malformed burn instruction data")]
    InvalidBurnIxData,
    #[msg("Burn mint does not match pool LP mint")]
    BurnMintMismatch,
    #[msg("Burn owner does not match withdrawer")]
    BurnOwnerMismatch,
}
