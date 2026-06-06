# instruction-introspection

A minimal constant-product AMM (Uniswap-v2 style) on Solana, built with Anchor.
Its defining feature: **withdraw uses instruction introspection** — it does not burn
LP tokens itself. Instead it reads the *preceding* instruction in the same
transaction off the instructions sysvar, verifies it is a real SPL-Token burn of the
pool's LP mint by the withdrawer, and pays out against that.

## Program ID

```
8qoU6VhEPekaod9EmVxU8BKND6SPfxhvWF2gaWhHAqgH
```

## Accounts / State

`AmmConfig` (PDA, one per `seed`) holds the pool:

| field | meaning |
|-------|---------|
| `seed` | distinguishes pools; part of the config PDA |
| `authority` | admin (defaults to initializer) |
| `mint_a` / `mint_b` | the pair (always `mint_a < mint_b`) |
| `mint_lp` | LP mint, authority = config PDA |
| `fee` | swap fee in basis points (e.g. `30` = 0.30%) |
| `locked` | circuit breaker |
| `bump` / `lp_bump` | PDA bumps |
| `reserve_a` / `reserve_b` | tracked reserves (payout source for withdraw) |

### PDAs

| account | seeds |
|---------|-------|
| `config` | `[CONFIG_SEED, seed.to_le_bytes()]` |
| `mint_lp` | `[LP_SEED, config.key()]` |
| `vault_a` / `vault_b` | ATAs of `config` for each mint |

## Instructions

### `initialize(seed, fee, authority)`
Creates the config PDA, both vaults, and the LP mint. Stores `fee`, sets reserves to 0.

### `deposit(amount, max_a, max_b)`
Adds liquidity, mints `amount` LP to the user.
- First deposit (`lp_supply == 0`): takes `max_a` / `max_b` as-is, setting the price.
- Later deposits: pulls `x = amount * reserve_a / lp_supply`, `y` likewise (proportional).
- `max_a` / `max_b` are **ceilings** — revert with `SlippageExceeded` if the pool needs more.

### `swap(side, amount_in, min_out)`
Constant-product swap in either direction (`Side::a` = give A get B, `Side::b` = reverse).
- Fee applied on input: `in' = amount_in * (10_000 - fee) / 10_000`.
- Output: `out = reserve_out - k / (reserve_in + in')`, `k = reserve_in * reserve_out`.
- `min_out` is a **floor** — revert if the pool would return less.

### `withdraw(min_a, min_b)` — the introspection instruction
Does **not** burn LP. The caller must place a standard SPL-Token `Burn` (or `BurnChecked`)
of the LP mint **immediately before** this instruction in the same transaction:

```
tx = [ SPL Token Burn(lp_amount) , withdraw(min_a, min_b) ]
```

`withdraw` then:
1. Reads the previous instruction via `load_current_index_checked` / `load_instruction_at_checked`.
2. Verifies: program is SPL Token, data tag is `Burn (8)` / `BurnChecked (15)`,
   `accounts[1] == mint_lp`, `accounts[2] == user` and is a signer.
3. Parses the burned amount from the burn instruction's data (`u64` LE at bytes `1..9`).
4. Pays out `burned * reserve / supply_before` of each token (rounded down),
   where `supply_before = mint_lp.supply + burned` (the burn already reduced supply).
5. `min_a` / `min_b` are **floors**.

This makes the burn a plain, composable SPL instruction any wallet/SDK can build —
the program merely *reacts* to it rather than performing it.

## Build & test

```bash
anchor build            # or: cargo build-sbf  — produces target/deploy/*.so
cargo test -p instruction-introspection
```

Tests run on [LiteSVM](https://github.com/LiteSVM/litesvm) against the compiled `.so`
and cover every instruction (`tests/test_initialize.rs`):

- `test_initialize` — pool + vaults + LP mint created.
- `test_deposit` — vaults funded, LP minted, user debited.
- `test_swap` — A→B swap moves balances; output < input (fee + curve).
- `test_withdraw_with_prior_burn` — burn + withdraw returns proportional share.
- `test_withdraw_without_burn_fails` — withdraw alone is rejected (introspection guard).

## Known limitations

- **`swap` does not update `config.reserve_a/b`.** Withdraw pays out from those cached
  reserves, so after a swap they drift from the true vault balances. Either have `swap`
  maintain the cache, or switch withdraw to read `vault.amount` directly. Pick one model.
- **Burn↔withdraw are linked only by position.** Two `withdraw`s after one `burn` could
  both reference the same burn. The transaction builder must keep them 1:1 (or the program
  needs an explicit anti-replay guard).
