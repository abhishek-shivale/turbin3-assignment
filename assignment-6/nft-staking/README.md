# NFT Staking

Anchor program for staking Metaplex Core (mpl-core) NFTs and earning SPL token rewards. Built on Solana with the mpl-core plugin system — staking state lives in the asset's on-chain attributes, and a `FreezeDelegate` plugin locks the NFT while staked.

- **Program ID:** `J3YnvbaxuLdTRrdf4eUoAPnS9zPAnwAsxcHCun5iPSun`
- **Anchor:** 0.31.1
- **NFT standard:** Metaplex Core (`CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d`)
- **Reward token:** SPL Token-2022 mint, 6 decimals

## How it works

Each NFT collection gets its own staking config and reward mint. Staking freezes the asset in place (owner keeps custody, asset can't transfer) and writes timestamp attributes onto it. Rewards accrue per day staked and are minted on `claim_reward` or `unstake`.

State is stored two ways:
- **`StakeState` PDA** — per-collection config (`rewards_bps`, `freeze_period`, bumps).
- **mpl-core attributes** — staking flags on the asset (`staked`, `staked_at`, `last_claimed_at`) and a running `staked_count` on the collection.

### Reward formula

```
amount = staked_days * rewards_bps * 10^decimals / 10_000
```

`staked_days` = whole days since `last_claimed_at`. `rewards_bps` is set at init (10_000 bps = 1 token/day at full rate).

## PDAs

| PDA | Seeds | Purpose |
|-----|-------|---------|
| `stake_state` | `["stake_state", collection]` | Per-collection staking config |
| `update_authority` | `["update_authority", collection]` | Signs mpl-core CPIs (collection + asset authority) |
| `rewards_mint` | `["rewards_mint", stake_state]` | Reward token mint; mint authority = `stake_state` |

## Instructions

| Instruction | Args | Description |
|-------------|------|-------------|
| `create_collection` | `name`, `uri` | Create mpl-core collection with `staked_count: 0` attribute, PDA update authority |
| `initialize` | `rewards_bps`, `freeze_period` | Create `StakeState` + rewards mint for the collection |
| `mint_asset` | `name`, `uri` | Mint an mpl-core asset into the collection to the user |
| `stake` | — | Freeze asset, set `staked/staked_at/last_claimed_at`, increment collection `staked_count` |
| `claim_reward` | — | Mint rewards for elapsed days, reset `last_claimed_at`, asset stays staked |
| `unstake` | — | Check freeze period elapsed, mint remaining rewards, unfreeze, decrement `staked_count` |

### Lifecycle

```
create_collection -> initialize -> mint_asset -> stake -> [claim_reward ...] -> unstake
```

`freeze_period` (days) gates `unstake` — asset can't be unstaked until that many days pass since `last_claimed_at`.

## Errors

`InvalidOwner`, `InvalidUpdateAuthority`, `AlreadyStaked`, `AssetNotStaked`, `InvalidTimestamp`, `FreezePeriodNotElapsed`, `InvalidRewardsBPS`, `FrozenAsset`, `MissingAttributes`, `MissingStakedCount`, `InvalidCollection`.

## Build & test

Tests run under [LiteSVM](https://github.com/LiteSVM/litesvm) and require two `.so` files in `target/deploy/`.

```bash
# 1. build the program
anchor build --ignore-keys

# 2. dump mpl-core from mainnet (tests load it into LiteSVM)
solana program dump -u m \
  CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d \
  target/deploy/mpl_core.so

# 3. run tests
cargo test
```

Test suites in `programs/nft-staking/tests/`: `test_create_collection`, `test_initialize`, `test_mint_asset`, `test_stake`, `test_unstake`, `test_claim_reward`. Shared fixtures in `helpers.rs`.

## Layout

```
programs/nft-staking/src/
├── lib.rs              # program entrypoints
├── constants.rs        # PDA seeds, attribute keys, mint decimals
├── state.rs            # StakeState account, MplCore program id
├── error.rs            # StakingError codes
└── instructions/       # create_collection, initialize, mint_asset, stake, unstake, claim_reward
```
