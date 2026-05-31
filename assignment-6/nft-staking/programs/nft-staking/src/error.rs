use anchor_lang::prelude::*;

#[error_code]
pub enum StakingError {
    #[msg("NFT owner dose not match the signer")]
    InvalidOwner,
    #[msg(" Invalid update authority for the NFT")]
    InvalidUpdateAuthority,
    #[msg("Asset is already stack")]
    AlreadyStaked,
    #[msg("Asset is not currently staked")]
    AssetNotStaked,
    #[msg("Provided timestamp id Invalid")]
    InvalidTimestamp,
    #[msg("Freeze Period has not elapsed yet")]
    FreezePeriodNotElapsed,
    #[msg("Reward basis points vaule id invalid")]
    InvalidRewardsBPS,
    #[msg("Asset is alredy frozen")]
    FrozenAsset,
    #[msg("Required asset attributes plugin is missing")]
    MissingAttributes,
    #[msg("Collection staking count attribute is missing")]
    MissingStakedCount,
    #[msg("Collection staking state in invalid")]
    InvalidCollection
}
