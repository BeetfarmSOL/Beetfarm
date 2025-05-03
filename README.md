<p align="center">
  <img src="https://i.imgur.com/wEDDHz7.png" alt="Beetfarm Banner">
</p>

<p align="center">
  <a href="https://beetfarm.net/">Website</a> •
  <a href="https://x.com/BeetfarmSOL">Twitter</a> •
  <a href="https://t.me/Beetfarm">Telegram</a>
</p>


# 🧃 Beetfarm

**The Beetfarm Protocol** is a Solana Anchor smart contract for managing token staking with reward multipliers based on stake duration and vault performance. It supports multiple stakers, custom base reward rates, and secure token transfers.

---

## 🚀 Features

- 📦 Vault Creation with base reward configuration  
- 🪙 Token Staking with tracking per user and time  
- 💸 Reward Calculation based on pool health and hold duration  
- 🧾 Partial or Full Withdrawals (with `reward_only` flag)  
- 🔐 Secure SPL token transfers via CPI  

---




## 🛠️ Program Structure
```
├── src/
│   ├── lib.rs             # Entry point for instructions
│   ├── constants.rs       # Global constants (e.g., seeds, time units)
│   ├── error.rs           # Custom error definitions
│   ├── helpers.rs         # Logic for vault ops (init, deposit, withdraw)
```

## 📋 Usage

### 📌 Create a Vault

```ts
await program.methods
  .createBeetVault(new BN(1_000_000), 0.1, 6)
  .accounts({
    vault: vaultPda,
    tokenAccount: tokenVaultPda,
    creator: user.publicKey,
    creatorTokenAccount: userTokenAccount,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .rpc();
```

* `amount`: Initial pool amount
* `base_rate`: % multiplier per interval (e.g. `0.1` = 10%)
* `base_hour`: Reward interval in hours

---

### 💰 Deposit Tokens

```ts
await program.methods
  .depositBeets(new BN(100_000), 0)
  .accounts({
    vault: vaultPda,
    userInteractionsCounter: userIxPda,
    vaultTokenAccount: tokenVaultPda,
    depositor: user.publicKey,
    depositorTokenAccount: userTokenAccount,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .rpc();
```

* `amount`: Token amount to deposit
* `index`: Deposit index (used to track each deposit separately)

---

### 🏦 Withdraw (Rewards or Full)

```ts
await program.methods
  .withdrawBeets(0, false)
  .accounts({
    vault: vaultPda,
    userInteractionsCounter: userIxPda,
    vaultTokenAccount: tokenVaultPda,
    withdrawer: user.publicKey,
    withdrawerTokenAccount: userTokenAccount,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .rpc();
```

* `index`: Index of the deposit to withdraw from
* `reward_only`: `true` for reward only, `false` to withdraw full deposit + rewards

---

## 🧠 How Rewards Work

1. Each deposit is timestamped.
2. On withdrawal, the elapsed time is compared to `base_hour`.
3. The pool health (remaining/initial) adjusts the multiplier.
4. Rewards are calculated as:

```
reward = deposit_amount * (pool_ratio) * (base_rate * periods_elapsed)
```

* Max reward duration: 24 hours

---

## 🧩 Account Overview

### `Vault`

* `amount`: Remaining tokens for reward
* `amount_staked`: Total currently staked
* `base_rate`: % reward multiplier per interval
* `base_hour`: Interval (in hours) for rewards
* `start_pool`: Initial token pool size

### `UserInteractionsCounter`

* `total_deposits`: \[u64; N] – User deposits
* `time_deposits`: \[u64; N] – Timestamps
* `stake_deposits`: \[u64; N] – For reward tracking
