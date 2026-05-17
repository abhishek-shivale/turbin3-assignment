# Escrow

Solana program for trustless token swaps between two parties. Built with Anchor.

A maker locks `amount` of `mint_a` in a vault and specifies how much `mint_b` they want in return. Any taker can fulfill the trade by sending `mint_b` to the maker, in exchange for the vaulted `mint_a`. Maker can refund anytime before a taker fills.

## Program ID

```
BxL739uVwWYBgxHzFRXLzKUkMVCiuVxxg8v997TTaGiJ
```

## Architecture

### State

`Escrow` account (PDA, seeds: `["escrow", maker, seed]`):

| Field    | Type     | Purpose                                  |
| -------- | -------- | ---------------------------------------- |
| `seed`   | `u64`    | Allows one maker to open multiple escrows |
| `maker`  | `Pubkey` | Original creator                          |
| `mint_a` | `Pubkey` | Token offered                             |
| `mint_b` | `Pubkey` | Token requested                           |
| `receive`| `u64`    | Amount of `mint_b` maker wants            |
| `bump`   | `u8`     | PDA bump                                  |

`vault` = ATA of `escrow` PDA holding `mint_a` tokens.

### Instructions

| Instruction | Signer  | Effect                                                                                          |
| ----------- | ------- | ----------------------------------------------------------------------------------------------- |
| `make`      | maker   | Init escrow PDA + vault ATA. Transfer `amount` of `mint_a` from maker into vault.               |
| `take`      | taker   | Send `receive` of `mint_b` from taker to maker. Transfer vault's `mint_a` to taker. Close vault + escrow. |
| `refund`    | maker   | Return vault's `mint_a` to maker. Close vault + escrow.                                         |

### Account layouts

Each instruction's full account list is in [`programs/escrow/src/instructions/`](programs/escrow/src/instructions/).

## Project structure

```
escrow/
├── Anchor.toml
├── Cargo.toml
├── programs/escrow/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs            # Program entry, instruction dispatch
│   │   ├── state.rs          # Escrow account struct
│   │   ├── error.rs          # Custom errors
│   │   └── instructions/
│   │       ├── make.rs
│   │       ├── take.rs
│   │       └── refund.rs
│   └── tests/
│       └── test_initialize.rs  # LiteSVM integration tests
└── target/                   # Build artifacts (gitignored)
```

## Prerequisites

- Rust + Cargo (`rustup`)
- Solana CLI (`solana --version`)
- Anchor CLI 1.0+ (`anchor --version`)
- Yarn (`yarn --version`)

## Build

```bash
anchor build
```

Produces `target/deploy/escrow.so` and `target/idl/escrow.json`.

## Test

Tests use [`litesvm`](https://github.com/LiteSVM/litesvm) — in-process Solana VM, no validator needed. Run:

```bash
anchor build              # tests load the compiled .so
cargo test --manifest-path programs/escrow/Cargo.toml
```

Test cases ([`test_initialize.rs`](programs/escrow/tests/test_initialize.rs)):

- `test_make` — maker creates escrow, vault funded.
- `test_refund` — maker reclaims tokens, escrow + vault closed.
- `test_take` — taker fulfills trade, both parties receive correct tokens, escrow + vault closed.

## Deploy

Local validator:
```bash
solana-test-validator
anchor deploy --provider.cluster localnet
```

Devnet:
```bash
anchor deploy --provider.cluster devnet
```

## Usage flow

```
1. Maker calls `make(seed, receive, amount)`
   → escrow PDA created, vault funded with `amount` of mint_a

2a. Taker calls `take()`
    → taker sends `receive` of mint_b to maker
    → taker receives `amount` of mint_a from vault
    → escrow + vault closed, rent returned to maker

2b. OR maker calls `refund()`
    → vault returns `amount` of mint_a to maker
    → escrow + vault closed, rent returned to maker
```

## Security notes

- Escrow PDA seed includes `maker` pubkey + `seed` u64 → maker can run many escrows in parallel without collision.
- Vault authority = escrow PDA. Only program can sign for vault.
- `has_one` constraints on escrow enforce `maker`, `mint_a`, `mint_b` match stored state.
- Refund signer = original maker only.
- Take signer = any taker. Trade is permissionless once made.

## License

ISC
