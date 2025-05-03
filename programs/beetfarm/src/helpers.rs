use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use std::convert::TryInto;

use crate::constants::*;
use crate::error::ErrorCode;
use crate::state::*;

pub fn iter_all_eq<T: PartialEq>(iter: impl IntoIterator<Item = T>) -> Option<T> {
    let mut iter = iter.into_iter();
    let first = iter.next()?;
    iter.all(|elem| elem == first).then(|| first)
}

pub fn initialize_vault(
    ctx: Context<CreateVault>,
    amount: u64,
    base_rate: f32,
    base_hour: u32,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;

    vault.amount = amount;
    vault.amount_staked = 0;
    vault.start_pool = amount;
    vault.base_rate = base_rate;
    vault.base_hour = base_hour;
    vault.total_stakers = 0;
    vault.current_stakers = 0;

    let cpi_accounts = Transfer {
        from: ctx.accounts.creator_token_account.to_account_info(),
        to: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.creator.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    token::transfer(CpiContext::new(cpi_program, cpi_accounts), amount)?;

    Ok(())
}

pub fn process_deposit(ctx: Context<Deposit>, amount: u64, index: usize) -> Result<()> {
    let clock = Clock::get()?;

    let total_deposits = &mut ctx.accounts.user_interactions_counter.total_deposits;
    if let Some(total) = total_deposits.get_mut(index) {
        *total = amount;
    } else {
        return Err(ErrorCode::InvalidIndex.into());
    }

    let time_deposits = &mut ctx.accounts.user_interactions_counter.time_deposits;
    if let Some(time) = time_deposits.get_mut(index) {
        *time = clock.unix_timestamp.try_into().map_err(|_| ErrorCode::Overflow)?;
    } else {
        return Err(ErrorCode::InvalidIndex.into());
    }

    let stake_deposits = &mut ctx.accounts.user_interactions_counter.stake_deposits;
    if let Some(stake) = stake_deposits.get_mut(index) {
        *stake = clock.unix_timestamp.try_into().map_err(|_| ErrorCode::Overflow)?;
    } else {
        return Err(ErrorCode::InvalidIndex.into());
    }

    ctx.accounts.vault.amount_staked = ctx
        .accounts
        .vault
        .amount_staked
        .checked_add(amount)
        .ok_or(ErrorCode::Overflow)?;

    let cpi_accounts = Transfer {
        from: ctx.accounts.depositor_token_account.to_account_info(),
        to: ctx.accounts.vault_token_account.to_account_info(),
        authority: ctx.accounts.depositor.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    token::transfer(CpiContext::new(cpi_program, cpi_accounts), amount)?;

    Ok(())
}

pub fn process_withdrawal(
    ctx: Context<Withdraw>,
    index: usize,
    reward_only: bool,
) -> Result<()> {
    let total_deposits = &ctx.accounts.user_interactions_counter.total_deposits;
    let amount = total_deposits.get(index).ok_or(ErrorCode::InvalidIndex)?;

    let stake_deposits = &ctx.accounts.user_interactions_counter.stake_deposits;
    let stake_time = stake_deposits.get(index).ok_or(ErrorCode::InvalidIndex)?;

    let clock = Clock::get()?;
    let unix_timestamp: u64 = clock
        .unix_timestamp
        .try_into()
        .map_err(|_| ErrorCode::Overflow)?;
    let time_elapsed = unix_timestamp
        .checked_sub(*stake_time)
        .ok_or(ErrorCode::Overflow)?
        .checked_mul(MILLISECONDS_IN_SECOND)
        .ok_or(ErrorCode::Overflow)?;

    let counters = &mut ctx.accounts.user_interactions_counter;
    if !reward_only {
        if let Some(total) = counters.total_deposits.get_mut(index) {
            *total = 0;
        } else {
            return Err(ErrorCode::InvalidIndex.into());
        }

        if let Some(time) = counters.time_deposits.get_mut(index) {
            *time = 0;
        } else {
            return Err(ErrorCode::InvalidIndex.into());
        }

        if let Some(stake) = counters.stake_deposits.get_mut(index) {
            *stake = 0;
        } else {
            return Err(ErrorCode::InvalidIndex.into());
        }
    } else {
        if let Some(stake) = counters.stake_deposits.get_mut(index) {
            *stake = unix_timestamp;
        } else {
            return Err(ErrorCode::InvalidIndex.into());
        }
    }

    if let Some(first_deposit) = total_deposits.get(0) {
        if *first_deposit == 0 && iter_all_eq(total_deposits).is_some() {
            ctx.accounts.vault.current_stakers = ctx
                .accounts
                .vault
                .current_stakers
                .checked_sub(1)
                .ok_or(ErrorCode::Overflow)?;
        }
    }

    let mut withdraw_amount = *amount;

    let hours_elapsed: u64 = time_elapsed
        .checked_div(MILLISECONDS_IN_SECOND)
        .and_then(|ms| ms.checked_div(SECONDS_IN_MINUTE))
        .and_then(|min| min.checked_div(MINUTES_IN_HOUR))
        .ok_or(ErrorCode::Overflow)?;

    if hours_elapsed >= ctx.accounts.vault.base_hour.try_into().map_err(|_| ErrorCode::Overflow)? {
        let start_pool_f64: f64 = ctx.accounts.vault.start_pool
            .try_into()
            .map_err(|_| ErrorCode::Overflow)?;

        if start_pool_f64 == 0.0 {
            return Err(ErrorCode::DivisionByZero.into());
        }

        let amount_f64: f64 = ctx.accounts.vault.amount
            .try_into()
            .map_err(|_| ErrorCode::Overflow)?;

        let pool_percent = amount_f64 / start_pool_f64;

        let base_hour_adjuster: u64 = ctx.accounts
            .vault
            .base_hour
            .try_into()
            .map_err(|_| ErrorCode::Overflow)?;

        let held_hours = hours_elapsed.min(MAX_HELD_HOURS);
        let remainder = held_hours % base_hour_adjuster;

        let multi: u64 = if remainder == 0 {
            held_hours.checked_div(base_hour_adjuster).ok_or(ErrorCode::Overflow)?
        } else {
            (held_hours.checked_sub(remainder).ok_or(ErrorCode::Overflow)?)
                .checked_div(base_hour_adjuster)
                .ok_or(ErrorCode::Overflow)?
        };

        let base_rate_f32: f32 = ctx.accounts.vault.base_rate
            .try_into()
            .map_err(|_| ErrorCode::Overflow)?;
        let final_multi: f32 = (multi.try_into().map_err(|_| ErrorCode::Overflow)? as f32 * base_rate_f32)
            .checked_mul(pool_percent as f32)
            .ok_or(ErrorCode::Overflow)?;

        let reward_tokens: u64 = (amount_f64 * (final_multi as f64))
            .floor()
            .try_into()
            .map_err(|_| ErrorCode::Overflow)?;

        ctx.accounts.vault.amount = ctx
            .accounts
            .vault
            .amount
            .checked_sub(reward_tokens)
            .ok_or(ErrorCode::Overflow)?;

        withdraw_amount = if reward_only {
            reward_tokens
        } else {
            withdraw_amount.checked_add(reward_tokens).ok_or(ErrorCode::Overflow)?
        };
    }

    if !reward_only {
        ctx.accounts.vault.amount_staked = ctx
            .accounts.vault.amount_staked
            .checked_sub(*amount)
            .ok_or(ErrorCode::Overflow)?;
    } else if withdraw_amount == *amount {
        withdraw_amount = 0;
    }

    if withdraw_amount > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.withdrawer_token_account.to_account_info(),
            authority: ctx.accounts.vault_token_account.to
::contentReference[oaicite:17]{index=17}