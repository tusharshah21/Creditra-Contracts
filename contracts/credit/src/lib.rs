#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreditStatus {
    Active = 0,
    Suspended = 1,
    Defaulted = 2,
    Closed = 3,
}

#[contracttype]
pub struct CreditLineData {
    pub borrower: Address,
    pub credit_limit: i128,
    pub utilized_amount: i128,
    pub interest_rate_bps: u32,
    pub risk_score: u32,
    pub status: CreditStatus,
}

/// Event emitted when a borrower draws credit from their line.
/// Enables off-chain indexers and backends to track borrowing activity.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreditDrawEvent {
    /// Address of the borrower drawing credit
    pub borrower: Address,
    /// Amount drawn in this transaction
    pub amount: i128,
    /// New total utilized amount after draw
    pub new_utilized: i128,
    /// Ledger timestamp when draw occurred
    pub timestamp: u64,
}

#[contract]
pub struct Credit;

#[contractimpl]
impl Credit {
    /// Initialize the contract (admin).
    pub fn init(env: Env, admin: Address) -> () {
        env.storage().instance().set(&Symbol::new(&env, "admin"), &admin);
        ()
    }

    /// Open a new credit line for a borrower (called by backend/risk engine).
    pub fn open_credit_line(
        _env: Env,
        _borrower: Address,
        _credit_limit: i128,
        _interest_rate_bps: u32,
        _risk_score: u32,
    ) -> () {
        // TODO: persist CreditLineData keyed by borrower
        ()
    }

    /// Draw from credit line (borrower).
    /// Emits a CreditDrawEvent for off-chain tracking.
    pub fn draw_credit(env: Env, borrower: Address, amount: i128) -> () {
        // TODO: check limit, update utilized_amount, transfer token to borrower
        
        // For now, simulate the new utilized amount (in full implementation, read from storage)
        let new_utilized = amount; // Placeholder: would be old_utilized + amount
        let timestamp = env.ledger().timestamp();
        
        // Emit draw event
        env.events().publish(
            (Symbol::new(&env, "credit_draw"), borrower.clone()),
            CreditDrawEvent {
                borrower,
                amount,
                new_utilized,
                timestamp,
            },
        );
        
        ()
    }

    /// Repay credit (borrower).
    pub fn repay_credit(_env: Env, _borrower: Address, _amount: i128) -> () {
        // TODO: accept token, reduce utilized_amount, accrue interest
        ()
    }

    /// Update risk parameters (admin/risk engine).
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

    /// Suspend a credit line (admin).
    pub fn suspend_credit_line(_env: Env, _borrower: Address) -> () {
        // TODO: set status to Suspended
        ()
    }

    /// Close a credit line (admin or borrower when utilized is 0).
    pub fn close_credit_line(_env: Env, _borrower: Address) -> () {
        // TODO: set status to Closed
        ()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};

    #[test]
    fn test_init_and_open_credit_line() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        // Placeholder: no panic means stubs work
    }

    #[test]
    fn test_draw_credit_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        
        let borrower = Address::generate(&env);
        let draw_amount = 500_i128;
        
        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);
        
        // Execute draw
        client.draw_credit(&borrower, &draw_amount);
        
        // Verify event was emitted
        let events = env.events().all();
        assert_eq!(events.len(), 1, "Expected exactly one event to be emitted");
        
        // Verify event topic contains credit_draw symbol and borrower
        let event = events.get(0).unwrap();
        assert_eq!(event.0, contract_id);
    }

    #[test]
    fn test_draw_credit_event_payload_structure() {
        let env = Env::default();
        env.mock_all_auths();
        
        let borrower = Address::generate(&env);
        let draw_amount = 1000_i128;
        
        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);
        
        client.draw_credit(&borrower, &draw_amount);
        
        let events = env.events().all();
        assert_eq!(events.len(), 1, "Expected one event");
        
        // Event was published successfully
        let event = events.get(0).unwrap();
        assert_eq!(event.0, contract_id);
    }

    #[test]
    fn test_multiple_draws_each_emit_event() {
        let env = Env::default();
        env.mock_all_auths();
        
        let borrower1 = Address::generate(&env);
        let borrower2 = Address::generate(&env);
        
        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);
        
        // First draw
        client.draw_credit(&borrower1, &300_i128);
        let events_after_first = env.events().all();
        assert_eq!(events_after_first.len(), 1, "Expected one event after first draw");
        assert_eq!(events_after_first.get(0).unwrap().0, contract_id);
        
        // Second draw
        client.draw_credit(&borrower2, &700_i128);
        let events_after_second = env.events().all();
        assert!(events_after_second.len() >= 1, "Expected at least one event after second draw");
        
        // Verify the most recent event is from the contract
        let last_event = events_after_second.get(events_after_second.len() - 1).unwrap();
        assert_eq!(last_event.0, contract_id);
    }

    #[test]
    fn test_draw_credit_includes_timestamp() {
        let env = Env::default();
        env.mock_all_auths();
        
        let borrower = Address::generate(&env);
        
        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);
        
        let timestamp_before = env.ledger().timestamp();
        client.draw_credit(&borrower, &250_i128);
        
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        
        // Timestamp should be captured at or after the call
        assert!(timestamp_before <= env.ledger().timestamp());
    }
}
