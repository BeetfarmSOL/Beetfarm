use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Overflow error")]
    Overflow,
    #[msg("Invalid index")]
    InvalidIndex,
    #[msg("Division by zero")]
    DivisionByZero,
}