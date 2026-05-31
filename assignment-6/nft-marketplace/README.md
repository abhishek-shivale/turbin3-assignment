# NFT Marketplace

An Anchor program for trading SPL-token NFTs with SOL or a configurable SPL payment token. Supports direct listings, escrowed bids (offers), a marketplace fee, and a rewards token minted to traders on each successful sale.

- **Program ID:** `A3SWqF5HhaZzvQ1VNyncN9u4AerDwatEUVTVVSE9Hrp4`
- **Anchor:** 0.31.1 · **Solana:** 2.x · **Tests:** [LiteSVM](https://github.com/LiteSVM/litesvm) 0.6

## Overview

A marketplace is created once by an admin. It owns a SOL fee **treasury**, a **rewards mint**, and accepts one configurable SPL **payment mint** for token-denominated trades. NFTs are escrowed in per-listing vaults; offers escrow the bidder's funds until accepted or cancelled.

## Accounts

| Account | PDA seeds | Purpose |
|---|---|---|
| `Marketplace` | `["marketplace", name]` | Config: admin, fee (bps), payment mint, bumps |
| `Listing` | `[marketplace, nft_mint]` | Maker, mint, price |
| `Offer` | `["offer", nft_mint, bidder]` | Bidder, mint, escrowed amount |
| treasury | `["treasury", marketplace]` | SOL fee vault (system account) |
| rewards mint | `["rewards", marketplace]` | Reward token, mint authority = marketplace PDA |

The token fee treasury is the marketplace PDA's associated token account for the payment mint. Each listing escrows its NFT in an ATA owned by the `Listing` PDA; each token offer escrows funds in an ATA owned by the `Offer` PDA.

## Instructions

| Instruction | Args | Effect |
|---|---|---|
| `init` | `name`, `fee` | Create marketplace, treasury, rewards mint |
| `list` | `price` | Create listing, escrow NFT into vault |
| `delist` | — | Return NFT to maker, close vault + listing |
| `buy_with_sol` | — | Pay maker in SOL (minus fee → treasury), send NFT, mint rewards to buyer |
| `buy_with_token` | — | Same paid in the payment SPL token (fee → token treasury) |
| `claim_sol_fee` | `amount` | Admin withdraws SOL fees from treasury |
| `claim_token_fee` | `amount` | Admin withdraws token fees from treasury |
| `make_sol_offer` | `amount` | Open an offer, escrow SOL in the offer PDA |
| `accept_sol_offer` | — | Pay acceptor (minus fee), transfer NFT to bidder, close offer |
| `cancel_sol_offer` | — | Refund escrow to bidder, close offer |
| `make_token_offer` | `amount` | Open an offer, escrow payment tokens in offer vault |
| `accept_token_offer` | — | Pay acceptor (minus fee), transfer NFT, mint rewards, close vault |
| `cancel_token_offer` | — | Refund escrowed tokens to bidder, close vault |

## Fees & rewards

- **Fee:** `fee` basis points of the sale price, capped at `10000` (100%). `fee = price * bps / 10_000`; the seller receives the remainder.
- **Rewards:** a fixed `1_000_000` units (1 token at 6 decimals) of the rewards mint is minted to the buyer (or, for offers, the acceptor) on each sale.

## Layout

```
programs/nft-marketplace/src/
  lib.rs                  # #[program] entrypoints
  state.rs                # Marketplace, Listing, Offer
  constants.rs            # PDA seeds, fee/reward constants
  error.rs                # MarketplaceError
  instructions/
    initialize.rs         # Init
    listing.rs            # List, Delist
    buy.rs                # BuyWithSol, BuyWithToken, shared mint_rewards
    claim.rs             # ClaimSolFee, ClaimTokenFee
    offer_sol.rs          # MakeSolOffer, AcceptSolOffer, CancelSolOffer
    offer_token.rs        # MakeTokenOffer, AcceptTokenOffer, CancelTokenOffer
```

## Build & test

```bash
anchor build                       # builds target/deploy/nft_marketplace.so
cargo test -p nft-marketplace      # runs the LiteSVM tests
```

Tests (`tests/marketplace.rs`) load the built `.so` into LiteSVM at runtime and cover `init`, `list`, `delist`, and `buy_with_sol` (NFT escrow, fee split, reward mint, account close).

> **Toolchain note:** the SBF build uses platform-tools Cargo 1.84 (pre-`edition2024`). `Cargo.lock` pins `proc-macro-crate = 3.1.0` and `blake3 = 1.5.5` to keep the dependency tree parseable. Running `cargo update` will re-pull `edition2024` crates and break `anchor build`.
