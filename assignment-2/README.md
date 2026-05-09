# Assignment 2 — Solana Token & NFT on Devnet

Rust programs that mint a Metaplex Core NFT and an SPL fungible token on Solana devnet.

---

## NFT — BlackPearl

**Standard:** Metaplex Core (MPL Core)  
**Asset address:** [`kVAC5wxemQoS2zBApeTezDQks78jRS8Psbf9WT4Z5WD`](https://core.metaplex.com/explorer/kVAC5wxemQoS2zBApeTezDQks78jRS8Psbf9WT4Z5WD?cluster=devnet&env=devnet)  
**Metadata URI:** `https://arweave.net/gfO_TkYttQls70pTmhrdMDz9pfMUXX8hZkaoIivQjGs` (Arweave)

### Plugins

| Plugin | Config |
|--------|--------|
| Royalties | 5% (500 bps), 100% to creator, no ruleset |
| FreezeDelegate | Owner-controlled, unfrozen at mint |

### Run

```bash
cd nft
cargo run
```

---

## SPL Token — Solana Gold (GOLDSOL)

**Standard:** SPL Token + Metaplex Token Metadata (Fungible)  
**Mint address:** [`4caYY634U3nTNtJcszEi9kTnKuMFPhdjvQRf3RPotPsU`](https://explorer.solana.com/address/4caYY634U3nTNtJcszEi9kTnKuMFPhdjvQRf3RPotPsU/metadata?cluster=devnet)

| Property | Value |
|----------|-------|
| Name | Solana Gold |
| Symbol | GOLDSOL |
| Decimals | 3 |
| Minted | 5.000 GOLDSOL |
| Transferred to recipient | 2.500 GOLDSOL |

All instructions bundled in one atomic transaction:
1. Create mint account
2. Initialize mint (3 decimals)
3. Attach Metaplex metadata
4. Create payer ATA
5. Create recipient ATA
6. Mint 5.000 GOLDSOL to payer
7. Transfer 2.500 GOLDSOL to recipient

### Run

```bash
cd spl
cargo run --bin spl
```

---

## Stack

- Rust + Tokio (async)
- `mpl-core` 0.12.0
- `mpl-token-metadata`
- `spl-token-interface` + `spl-associated-token-account-interface`
- `solana-sdk` / `solana-client`
- Network: **devnet**
