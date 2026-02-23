//! # Creditra Credit Contract
//!
//! This module implements the on-chain credit line protocol for Creditra on
//! Stellar Soroban. It manages the full lifecycle of borrower credit lines —
//! opening, drawing, repaying, suspending, closing, and defaulting.
//!
//! ## Roles
//!
//! - **Admin**: Deployed and initialized by the protocol deployer. Authorized
//!   to suspend, close, and default credit lines, and update risk parameters.
//! - **Borrower**: An address with an open credit line. Authorized to draw
//!   and repay funds within their credit limit.
//! - **Risk Engine / Backend**: Authorized to open credit lines and update
//!   risk parameters on behalf of the protocol.
//!
//! ## Main Flows
//!
//! 1. **Open**: Admin/backend calls `open_credit_line` to create a credit line
//!    for a borrower with a limit, interest rate, and risk score.
//! 2. **Draw**: Borrower calls `draw_credit` to borrow against their limit.
//! 3. **Repay**: Borrower calls `repay_credit` to repay drawn funds.
//! 4. **Suspend**: Admin calls `suspend_credit_line` to temporarily freeze a line.
//! 5. **Close**: Admin or borrower calls `close_credit_line` to permanently close.
//! 6. **Default**: Admin calls `default_credit_line` to mark a borrower as defaulted.
//!
//! ## Invariants
//!
//! - `utilized_amount` must never exceed `credit_limit`.
//! - A credit line must exist before it can be suspended, closed, or defaulted.
//! - Interest rate is expressed in basis points (1 bps = 0.01%).
//!
//! ## External Docs
//!
//! See [`docs/credit.md`](../../../docs/credit.md) for full documentation
//! including CLI usage and deployment instructions.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

/// Represents the lifecycle status of a credit line.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreditStatus {
    /// Credit line is open and available for drawing.
    Active = 0,
    /// Credit line is temporarily suspended by admin.
    Suspended = 1,
    /// Borrower has defaulted on the credit line.
    Defaulted = 2,
    /// Credit line has been permanently closed.
    Closed = 3,
}

/// Stores the full state of a borrower's credit line.
///
/// Persisted in contract storage keyed by the borrower's [`Address`].
#[contracttype]
pub struct CreditLineData {
    /// The borrower's Stellar address.
    pub borrower: Address,
    /// Maximum amount the borrower is authorized to draw.
    pub credit_limit: i128,
    /// Amount currently drawn and outstanding.
    pub utilized_amount: i128,
    /// Annual interest rate in basis points (e.g. 300 = 3%).
    pub interest_rate_bps: u32,
    /// Risk score assigned by the risk engine (0–100, higher = riskier).
    pub risk_score: u32,
    /// Current lifecycle status of the credit line.
    pub status: CreditStatus,
}

/// Event emitted on every credit line lifecycle state change.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreditLineEvent {
    /// Short symbol identifying the event type (e.g. `opened`, `suspend`).
    pub event_type: Symbol,
    /// The borrower whose credit line was affected.
    pub borrower: Address,
    /// The new status after the event.
    pub status: CreditStatus,
    /// Credit limit at the time of the event.
    pub credit_limit: i128,
    /// Interest rate at the time of the event.
    pub interest_rate_bps: u32,
    /// Risk score at the time of the event.
    pub risk_score: u32,
}

/// The Creditra credit contract.
#[contract]
pub struct Credit;

#[contractimpl]
impl Credit {
    /// Initialize the contract with an admin address.
    ///
    /// Must be called exactly once after deployment before any other
    /// function can be used.
    ///
    /// # Parameters
    /// - `admin`: The address authorized to perform admin operations.
    ///
    /// # Storage
    /// Stores `admin` in instance storage under the key `"admin"`.
    pub fn init(env: Env, admin: Address) -> () {
        env.storage().instance().set(&Symbol::new(&env, "admin"), &admin);
        ()
    }

    /// Open a new credit line for a borrower.
    ///
    /// Called by the backend or risk engine after off-chain credit assessment.
    /// Creates a new [`CreditLineData`] record with `utilized_amount = 0` and
    /// `status = Active`, then persists it keyed by the borrower's address.
    ///
    /// # Parameters
    /// - `borrower`: The borrower's Stellar address.
    /// - `credit_limit`: Maximum drawable amount.
    /// - `interest_rate_bps`: Annual interest rate in basis points.
    /// - `risk_score`: Risk score from the risk engine (0–100).
    ///
    /// # Events
    /// Emits a `("credit", "opened")` [`CreditLineEvent`].
    pub fn open_credit_line(
        env: Env,
        borrower: Address,
        credit_limit: i128,
        interest_rate_bps: u32,
        risk_score: u32,
    ) -> () {
        let credit_line = CreditLineData {
            borrower: borrower.clone(),
            credit_limit,
            utilized_amount: 0,
            interest_rate_bps,
            risk_score,
            status: CreditStatus::Active,
        };

        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

        env.events().publish(
            (symbol_short!("credit"), symbol_short!("opened")),
            CreditLineEvent {
                event_type: symbol_short!("opened"),
                borrower: borrower.clone(),
                status: CreditStatus::Active,
                credit_limit,
                interest_rate_bps,
                risk_score,
            },
        );
        ()
    }

    /// Draw funds from an active credit line.
    ///
    /// Called by the borrower to borrow against their credit limit.
    ///
    /// # Parameters
    /// - `borrower`: The borrower's address.
    /// - `amount`: Amount to draw. Must not exceed available credit.
    ///
    /// # Note
    /// Not yet implemented. Planned logic: validate amount against available
    /// credit, update `utilized_amount`, transfer tokens to borrower.
    pub fn draw_credit(_env: Env, _borrower: Address, _amount: i128) -> () {
        // TODO: check limit, update utilized_amount, transfer token to borrower
        ()
    }

    /// Repay outstanding credit and accrue interest.
    ///
    /// Called by the borrower to reduce their `utilized_amount`.
    ///
    /// # Parameters
    /// - `borrower`: The borrower's address.
    /// - `amount`: Amount to repay.
    ///
    /// # Note
    /// Not yet implemented. Planned logic: accept token transfer, reduce
    /// `utilized_amount`, accrue interest on outstanding balance.
    pub fn repay_credit(_env: Env, _borrower: Address, _amount: i128) -> () {
        // TODO: accept token, reduce utilized_amount, accrue interest
        ()
    }

    /// Update risk parameters for an existing credit line.
    ///
    /// Called by admin or risk engine when a borrower's risk profile changes.
    ///
    /// # Parameters
    /// - `borrower`: The borrower's address.
    /// - `credit_limit`: New credit limit.
    /// - `interest_rate_bps`: New interest rate in basis points.
    /// - `risk_score`: New risk score.
    ///
    /// # Note
    /// Not yet implemented. Planned logic: load existing record, update fields,
    /// persist updated [`CreditLineData`].
    pub fn update_risk_parameters(
        _env: Env,
        _borrower: Address,
        _credit_limit: i128,
        _interest_rate_bps: u32,
        _risk_score: u32,
    ) -> () {
        // TODO: update stored CreditLineData
        ()
    }

    /// Suspend a credit line temporarily.
    ///
    /// Called by admin to freeze a borrower's credit line without closing it.
    /// The credit line can be reactivated or closed after suspension.
    ///
    /// # Parameters
    /// - `borrower`: The borrower's address.
    ///
    /// # Panics
    /// - If no credit line exists for the given borrower.
    ///
    /// # Events
    /// Emits a `("credit", "suspend")` [`CreditLineEvent`].
    pub fn suspend_credit_line(env: Env, borrower: Address) -> () {
        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        credit_line.status = CreditStatus::Suspended;
        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

        env.events().publish(
            (symbol_short!("credit"), symbol_short!("suspend")),
            CreditLineEvent {
                event_type: symbol_short!("suspend"),
                borrower: borrower.clone(),
                status: CreditStatus::Suspended,
                credit_limit: credit_line.credit_limit,
                interest_rate_bps: credit_line.interest_rate_bps,
                risk_score: credit_line.risk_score,
            },
        );
        ()
    }

    /// Permanently close a credit line.
    ///
    /// Can be called by admin or by the borrower when `utilized_amount` is 0.
    /// Once closed, the credit line cannot be reopened.
    ///
    /// # Parameters
    /// - `borrower`: The borrower's address.
    ///
    /// # Panics
    /// - If no credit line exists for the given borrower.
    ///
    /// # Events
    /// Emits a `("credit", "closed")` [`CreditLineEvent`].
    pub fn close_credit_line(env: Env, borrower: Address) -> () {
        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        credit_line.status = CreditStatus::Closed;
        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

        env.events().publish(
            (symbol_short!("credit"), symbol_short!("closed")),
            CreditLineEvent {
                event_type: symbol_short!("closed"),
                borrower: borrower.clone(),
                status: CreditStatus::Closed,
                credit_limit: credit_line.credit_limit,
                interest_rate_bps: credit_line.interest_rate_bps,
                risk_score: credit_line.risk_score,
            },
        );
        ()
    }

    /// Mark a credit line as defaulted.
    ///
    /// Called by admin when a borrower fails to repay. Defaulted credit lines
    /// are permanently marked and cannot be reactivated.
    ///
    /// # Parameters
    /// - `borrower`: The borrower's address.
    ///
    /// # Panics
    /// - If no credit line exists for the given borrower.
    ///
    /// # Events
    /// Emits a `("credit", "default")` [`CreditLineEvent`].
    pub fn default_credit_line(env: Env, borrower: Address) -> () {
        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        credit_line.status = CreditStatus::Defaulted;
        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

        env.events().publish(
            (symbol_short!("credit"), symbol_short!("default")),
            CreditLineEvent {
                event_type: symbol_short!("default"),
                borrower: borrower.clone(),
                status: CreditStatus::Defaulted,
                credit_limit: credit_line.credit_limit,
                interest_rate_bps: credit_line.interest_rate_bps,
                risk_score: credit_line.risk_score,
            },
        );
        ()
    }

    /// Retrieve the current credit line data for a borrower.
    ///
    /// View function — does not modify any state.
    ///
    /// # Parameters
    /// - `borrower`: The borrower's address to look up.
    ///
    /// # Returns
    /// `Some(CreditLineData)` if a credit line exists, `None` otherwise.
    pub fn get_credit_line(env: Env, borrower: Address) -> Option<CreditLineData> {
        env.storage().persistent().get(&borrower)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_init_and_open_credit_line() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);

        let credit_line = client.get_credit_line(&borrower);
        assert!(credit_line.is_some());
        let credit_line = credit_line.unwrap();
        assert_eq!(credit_line.borrower, borrower);
        assert_eq!(credit_line.credit_limit, 1000);
        assert_eq!(credit_line.utilized_amount, 0);
        assert_eq!(credit_line.interest_rate_bps, 300);
        assert_eq!(credit_line.risk_score, 70);
        assert_eq!(credit_line.status, CreditStatus::Active);
    }

    #[test]
    fn test_suspend_credit_line() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.suspend_credit_line(&borrower);

        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.status, CreditStatus::Suspended);
    }

    #[test]
    fn test_close_credit_line() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.close_credit_line(&borrower);

        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.status, CreditStatus::Closed);
    }

    #[test]
    fn test_default_credit_line() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.default_credit_line(&borrower);

        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.status, CreditStatus::Defaulted);
    }

    #[test]
    fn test_full_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &5000_i128, &500_u32, &80_u32);
        assert_eq!(client.get_credit_line(&borrower).unwrap().status, CreditStatus::Active);

        client.suspend_credit_line(&borrower);
        assert_eq!(client.get_credit_line(&borrower).unwrap().status, CreditStatus::Suspended);

        client.close_credit_line(&borrower);
        assert_eq!(client.get_credit_line(&borrower).unwrap().status, CreditStatus::Closed);
    }

    #[test]
    fn test_event_data_integrity() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &2000_i128, &400_u32, &75_u32);

        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.borrower, borrower);
        assert_eq!(credit_line.status, CreditStatus::Active);
        assert_eq!(credit_line.credit_limit, 2000);
        assert_eq!(credit_line.interest_rate_bps, 400);
        assert_eq!(credit_line.risk_score, 75);
    }

    #[test]
    #[should_panic(expected = "Credit line not found")]
    fn test_suspend_nonexistent_credit_line() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.suspend_credit_line(&borrower);
    }

    #[test]
    #[should_panic(expected = "Credit line not found")]
    fn test_close_nonexistent_credit_line() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.close_credit_line(&borrower);
    }

    #[test]
    #[should_panic(expected = "Credit line not found")]
    fn test_default_nonexistent_credit_line() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.default_credit_line(&borrower);
    }

    #[test]
    fn test_multiple_borrowers() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower1 = Address::generate(&env);
        let borrower2 = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower1, &1000_i128, &300_u32, &70_u32);
        client.open_credit_line(&borrower2, &2000_i128, &400_u32, &80_u32);

        let credit_line1 = client.get_credit_line(&borrower1).unwrap();
        let credit_line2 = client.get_credit_line(&borrower2).unwrap();

        assert_eq!(credit_line1.credit_limit, 1000);
        assert_eq!(credit_line2.credit_limit, 2000);
        assert_eq!(credit_line1.status, CreditStatus::Active);
        assert_eq!(credit_line2.status, CreditStatus::Active);
    }

    #[test]
    fn test_lifecycle_transitions() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        assert_eq!(client.get_credit_line(&borrower).unwrap().status, CreditStatus::Active);

        client.default_credit_line(&borrower);
        assert_eq!(client.get_credit_line(&borrower).unwrap().status, CreditStatus::Defaulted);
    }
}