# Vault

Solana program built with Anchor that lets users deposit and withdraw SOL into a personal PDA-based vault.

## Overview

Each user gets two PDAs:
- **VaultState** — stores bump seeds, seeded by `["state", user_pubkey]`
- **Vault** — holds lamports, seeded by `["vault", vault_state_pubkey]`

The vault is controlled by PDA signing, so only the program (on behalf of the user) can move funds out.

## Instructions

| Instruction  | Description |
|---|---|
| `initialize` | Creates VaultState and Vault PDAs for the caller |
| `deposit`    | Transfers SOL from user wallet into Vault |
| `withdraw`   | Transfers SOL from Vault back to user wallet |
| `close`      | Drains all SOL from Vault to user and closes VaultState (rent reclaimed) |

## Account Structure

```
VaultState {
    state_bump: u8,
    vault_bump: u8,
}
```

## PDA Seeds

```
vault_state = ["state", user.key()]
vault       = ["vault", vault_state.key()]
```

## Testing

Uses [LiteSVM](https://github.com/LiteSVM/litesvm) for fast native tests without spinning up a validator.

```bash
cargo test
```

## Build & Deploy

```bash
# Build
anchor build

# Deploy to localnet
anchor deploy

# Run tests
cargo test
```

## Program ID

```
8x4Jwd2A9YSnEo8eWujjPdkXspFZYTESUZS4zzGGbhMs
```

## Stack

- Rust / Anchor
- LiteSVM (testing)
- Solana (localnet)
