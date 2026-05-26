# Assignment 5 — Solana AMM (Constant Product)

Anchor-based automated market maker on Solana using a constant product curve (`x * y = k`).

## Program ID

```
6NvaFi7fwTf4nnK5sMJKRHgEKDyD5FZUTsE5cDb1ZQf2
```

## Architecture

```
programs/assignment-5/src/
├── instructions/
│   ├── initialize.rs   — create pool, mint LP token, vaults
│   ├── deposit.rs      — add liquidity, receive LP tokens
│   ├── withdraw.rs     — burn LP tokens, receive X/Y
│   └── swap.rs         — swap X→Y or Y→X
├── state.rs            — Config account
└── lib.rs
```

### Config Account

| Field          | Type            | Description                          |
|----------------|-----------------|--------------------------------------|
| `seed`         | `u64`           | Unique pool identifier               |
| `authority`    | `Option<Pubkey>`| Optional admin (None = permissionless)|
| `mint_x`       | `Pubkey`        | Token X mint                         |
| `mint_y`       | `Pubkey`        | Token Y mint                         |
| `fee`          | `u16`           | Fee in basis points (e.g. 30 = 0.3%)|
| `locked`       | `bool`          | Pool pause flag                      |
| `config_bump`  | `u8`            | PDA bump                             |
| `lp_bump`      | `u8`            | LP mint PDA bump                     |

### PDAs

```
config  → ["config", seed.to_le_bytes()]
mint_lp → ["lp", config_pubkey]
vault_x → ATA(config, mint_x)
vault_y → ATA(config, mint_y)
```

## Instructions

### `initialize(seed, fee, authority)`

Creates pool config, LP mint, and token vaults.

### `deposit(amount, max_x, max_y)`

Deposits tokens proportional to pool ratio. `amount` = LP tokens to mint. Reverts if required X/Y exceed `max_x`/`max_y` (slippage guard).

**First deposit** sets the initial price ratio by accepting `max_x`/`max_y` as-is.

### `withdraw(amount, min_x, min_y)`

Burns `amount` LP tokens, withdraws proportional X and Y. Reverts if returned amounts are below `min_x`/`min_y`.

### `swap(is_x, amount_in, min_amount_out)`

Swaps token X→Y (`is_x=true`) or Y→X (`is_x=false`). Fee deducted before curve calculation. Reverts if output < `min_amount_out`.

## Stack

| Crate                    | Version  |
|--------------------------|----------|
| `anchor-lang`            | 1.0.1    |
| `anchor-spl`             | 1.0.1    |
| `constant-product-curve` | git      |
| `litesvm`                | 0.10.0   |

## Build & Test

```bash
# Build
anchor build

# Run all tests
anchor test

# Run specific test suite
cargo test --test test_swap
cargo test --test test_deposit
cargo test --test test_withdraw
cargo test --test test_initialize
```

## Test Coverage

| Suite               | Cases                                                    |
|---------------------|----------------------------------------------------------|
| `test_initialize`   | basic init, with authority, duplicate seed fails         |
| `test_deposit`      | first deposit, second deposit, slippage exceeded, zero amount |
| `test_swap`         | X→Y, Y→X, slippage exceeded, zero amount fails          |
| `test_withdraw`     | basic withdraw, slippage exceeded, zero amount fails     |
