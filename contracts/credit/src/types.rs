//! Core data types for the Credit contract.

use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreditStatus {
    Active = 0,
    Suspended = 1,
    Defaulted = 2,
    Closed = 3,
}

#[soroban_sdk::contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    Unauthorized = 1,
    NotAdmin = 2,
    CreditLineNotFound = 3,
    CreditLineClosed = 4,
    InvalidAmount = 5,
    OverLimit = 6,
    NegativeLimit = 7,
    RateTooHigh = 8,
    ScoreTooHigh = 9,
    UtilizationNotZero = 10,
    Reentrancy = 11,
    Overflow = 12,
}

/// Stored credit line for a borrower.
#[contracttype]
pub struct CreditLineData {
    pub borrower: Address,
    pub credit_limit: i128,
    pub utilized_amount: i128,
    pub interest_rate_bps: u32,
    pub risk_score: u32,
    pub status: CreditStatus,
}
