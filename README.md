# Creditra Contracts

Soroban smart contracts for the Creditra adaptive credit protocol on Stellar.

## About

This repo contains the **credit** contract: it maintains credit lines, tracks utilization, enforces limits, and exposes methods for opening lines, drawing, repaying, and updating risk parameters. Draw logic includes a liquidity reserve check and token transfer flow.

**Contract data model:**

- `CreditStatus`: Active, Suspended, Defaulted, Closed
- `CreditLineData`: borrower, credit_limit, utilized_amount, interest_rate_bps, risk_score, status

**Methods:** `init`, `set_liquidity_token`, `set_liquidity_source`, `open_credit_line`, `draw_credit`, `repay_credit`, `update_risk_parameters`, `suspend_credit_line`, `close_credit_line`.

### Liquidity reserve enforcement

- `draw_credit` now checks configured liquidity token balance at the configured liquidity source before transfer.
- If reserve balance is less than requested draw amount, the transaction reverts with: `Insufficient liquidity reserve for requested draw amount`.
- `init` defaults liquidity source to the contract address.
- Admin can configure:
  - `set_liquidity_token` — token contract used for reserve and draw transfers.
  - `set_liquidity_source` — reserve address to fund draws (contract or external source).

## Tech Stack

- **Rust** (edition 2021)
- **soroban-sdk** (Stellar Soroban)
- Build target: **wasm32** for Soroban

## Prerequisites

- Rust 1.75+ (recommend latest stable)
- `wasm32` target:

  ```bash
  rustup target add wasm32-unknown-unknown
  ```

- [Stellar Soroban CLI](https://developers.stellar.org/docs/smart-contracts/getting-started/setup) for deploy and invoke (optional for local build).

## Setup and build

```bash
cd creditra-contracts
cargo build --release -p creditra-credit
```

WASM output (after a typical Soroban setup) is under `target/wasm32-unknown-unknown/release/creditra_credit.wasm` (name may vary by crate name).

### Run tests

```bash
cargo test -p creditra-credit
```

### Deploy (with Soroban CLI)

Once the Soroban CLI and a network are configured:

```bash
soroban contract deploy --wasm target/wasm32-unknown-unknown/release/creditra_credit.wasm --source <identity> --network <network>
```

See [Stellar Soroban docs](https://developers.stellar.org/docs/smart-contracts) for details.

## Project layout

- `Cargo.toml` — workspace and release profile (opt for contract size)
- `contracts/credit/` — credit line contract
  - `Cargo.toml` — crate config, soroban-sdk dependency
  - `src/lib.rs` — contract types and impl (stubs)

## Merging to remote

This repo is a standalone git repository. After adding your remote:

```bash
git remote add origin <your-creditra-contracts-repo-url>
git push -u origin main
```
