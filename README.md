# Creditra Contracts

Soroban smart contracts for the Creditra adaptive credit protocol on Stellar.

## About

This repo contains the **credit** contract: it will maintain credit lines, track utilization, enforce limits, and expose methods for opening lines, drawing, repaying, and updating risk parameters. The current code is a **stub** with the correct API and data types; full logic (storage, token transfers, interest) is to be implemented.

**Contract data model:**

- `CreditStatus`: Active, Suspended, Defaulted, Closed
- `CreditLineData`: borrower, credit_limit, utilized_amount, interest_rate_bps, risk_score, status

**Methods:** `init`, `open_credit_line`, `draw_credit`, `repay_credit`, `update_risk_parameters`, `suspend_credit_line`, `close_credit_line`.

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

### Overflow scenario tests (large amounts)

The credit contract includes dedicated overflow and large-value tests in
`contracts/credit/src/lib.rs`:

- `test_draw_credit_near_i128_max_succeeds_without_overflow`
- `test_draw_credit_overflow_reverts_with_defined_error`
- `test_draw_credit_large_values_exceed_limit_reverts_with_defined_error`

These tests validate that:

- near-`i128::MAX` draws succeed when within limit;
- arithmetic overflow reverts with the defined `"overflow"` panic;
- large-value over-limit draws revert with the defined `"exceeds credit limit"` panic.

### Coverage

Run coverage with:

```bash
cargo llvm-cov --workspace --all-targets --fail-under-lines 95
```

Current result:

- Regions: `99.51%`
- Lines: `98.94%`

This satisfies the 95% minimum coverage target.

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
