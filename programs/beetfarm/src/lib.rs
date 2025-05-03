use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use std::convert::TryInto;

declare_id!("XXXXXXXXXXXXXXXXXXXXXXXXXXCONTRACT");

mod constants;
mod error;
mod helpers;
mod state;

use constants::*;
use error::ErrorCode;
use helpers::*;
use state::*;

#[program]
pub mod beet_vault {
    use super::*;

    pub fn create_beet_vault(
        ctx: Context<CreateVault>,
        amount: u64,
        base_rate: f32,
        base_hour: u32,
    ) -> Result<()> {
        initialize_vault(ctx, amount, base_rate, base_hour)
    }

    pub fn deposit_beets(ctx: Context<Deposit>, amount: u64, index: usize) -> Result<()> {
        process_deposit(ctx, amount, index)
    }

    pub fn withdraw_beets(ctx: Context<Withdraw>, index: usize, reward_only: bool) -> Result<()> {
        process_withdrawal(ctx, index, reward_only)
    }
}
