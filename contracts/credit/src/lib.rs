#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

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

/// Event emitted when a credit line lifecycle event occurs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreditLineEvent {
    pub event_type: Symbol,
    pub borrower: Address,
    pub status: CreditStatus,
    pub credit_limit: i128,
    pub interest_rate_bps: u32,
    pub risk_score: u32,
}

#[contract]
pub struct Credit;

#[contractimpl]
impl Credit {
    /// Initialize the contract (admin).
    pub fn init(env: Env, admin: Address) -> () {
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "admin"), &admin);
        ()
    }

    /// Open a new credit line for a borrower (called by backend/risk engine).
    /// Emits a CreditLineOpened event.
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

        env.storage().persistent().set(&borrower, &credit_line);

        // Emit CreditLineOpened event
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

    /// Draw from credit line (borrower).
    /// Reverts if credit line does not exist, is Closed, or borrower has not authorized.
    pub fn draw_credit(env: Env, borrower: Address, amount: i128) -> () {
        borrower.require_auth();
        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");
        if credit_line.status == CreditStatus::Closed {
            panic!("credit line is closed");
        }
        if amount <= 0 {
            panic!("amount must be positive");
        }
        let new_utilized = credit_line
            .utilized_amount
            .checked_add(amount)
            .expect("overflow");
        if new_utilized > credit_line.credit_limit {
            panic!("exceeds credit limit");
        }
        credit_line.utilized_amount = new_utilized;
        env.storage().persistent().set(&borrower, &credit_line);
        // TODO: transfer token to borrower
        ()
    }

    /// Repay credit (borrower).
    /// Reverts if credit line does not exist, is Closed, or borrower has not authorized.
    pub fn repay_credit(env: Env, borrower: Address, _amount: i128) -> () {
        borrower.require_auth();
        let credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");
        if credit_line.status == CreditStatus::Closed {
            panic!("credit line is closed");
        }
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
    /// Emits a CreditLineSuspended event.
    pub fn suspend_credit_line(env: Env, borrower: Address) -> () {
        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        credit_line.status = CreditStatus::Suspended;
        env.storage().persistent().set(&borrower, &credit_line);

        // Emit CreditLineSuspended event
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

    /// Close a credit line. Callable by admin (force-close) or by borrower when utilization is zero.
    ///
    /// # Arguments
    /// * `closer` - Address that must have authorized this call. Must be either the contract admin
    ///   (can close regardless of utilization) or the borrower (can close only when
    ///   `utilized_amount` is zero).
    ///
    /// # Errors
    /// * Panics if credit line does not exist, or if `closer` is not admin/borrower, or if
    ///   borrower closes while `utilized_amount != 0`.
    ///
    /// Emits a CreditLineClosed event.
    pub fn close_credit_line(env: Env, borrower: Address, closer: Address) -> () {
        closer.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin"))
            .expect("admin not set");

        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        if credit_line.status == CreditStatus::Closed {
            return ();
        }

        let allowed = closer == admin || (closer == borrower && credit_line.utilized_amount == 0);

        if !allowed {
            if closer == borrower {
                panic!("cannot close: utilized amount not zero");
            }
            panic!("unauthorized");
        }

        credit_line.status = CreditStatus::Closed;
        env.storage().persistent().set(&borrower, &credit_line);

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

    /// Mark a credit line as defaulted (admin).
    /// Emits a CreditLineDefaulted event.
    pub fn default_credit_line(env: Env, borrower: Address) -> () {
        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        credit_line.status = CreditStatus::Defaulted;
        env.storage().persistent().set(&borrower, &credit_line);

        // Emit CreditLineDefaulted event
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

    /// Get credit line data for a borrower (view function).
    pub fn get_credit_line(env: Env, borrower: Address) -> Option<CreditLineData> {
        env.storage().persistent().get(&borrower)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::testutils::Events as _;

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

        // Verify credit line was created
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

        // Verify status changed to Suspended
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
        client.close_credit_line(&borrower, &admin);

        // Verify status changed to Closed
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

        // Verify status changed to Defaulted
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

        // Open credit line
        client.open_credit_line(&borrower, &5000_i128, &500_u32, &80_u32);
        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.status, CreditStatus::Active);

        // Suspend credit line
        client.suspend_credit_line(&borrower);
        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.status, CreditStatus::Suspended);

        // Close credit line
        client.close_credit_line(&borrower, &admin);
        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.status, CreditStatus::Closed);
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

        // Verify credit line data matches what was passed
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
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.close_credit_line(&borrower, &admin);
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

        // Test Active -> Defaulted
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        assert_eq!(
            client.get_credit_line(&borrower).unwrap().status,
            CreditStatus::Active
        );

        client.default_credit_line(&borrower);
        assert_eq!(
            client.get_credit_line(&borrower).unwrap().status,
            CreditStatus::Defaulted
        );
    }

    // --- close_credit_line: admin vs borrower, utilization ---

    #[test]
    fn test_close_credit_line_borrower_when_utilized_zero() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.close_credit_line(&borrower, &borrower);

        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.status, CreditStatus::Closed);
        assert_eq!(credit_line.utilized_amount, 0);
    }

    #[test]
    #[should_panic(expected = "cannot close: utilized amount not zero")]
    fn test_close_credit_line_borrower_rejected_when_utilized_nonzero() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.draw_credit(&borrower, &300_i128);

        client.close_credit_line(&borrower, &borrower);
    }

    #[test]
    fn test_close_credit_line_admin_force_close_with_utilization() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.draw_credit(&borrower, &300_i128);
        assert_eq!(
            client.get_credit_line(&borrower).unwrap().utilized_amount,
            300
        );

        client.close_credit_line(&borrower, &admin);

        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.status, CreditStatus::Closed);
        assert_eq!(credit_line.utilized_amount, 300);
    }

    #[test]
    fn test_close_credit_line_idempotent_when_already_closed() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.close_credit_line(&borrower, &admin);
        client.close_credit_line(&borrower, &admin);

        assert_eq!(
            client.get_credit_line(&borrower).unwrap().status,
            CreditStatus::Closed
        );
    }

    #[test]
    #[should_panic(expected = "credit line is closed")]
    fn test_draw_credit_rejected_when_closed() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.close_credit_line(&borrower, &admin);

        client.draw_credit(&borrower, &100_i128);
    }

    #[test]
    #[should_panic(expected = "credit line is closed")]
    fn test_repay_credit_rejected_when_closed() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.close_credit_line(&borrower, &admin);

        client.repay_credit(&borrower, &100_i128);
    }

    #[test]
    #[should_panic(expected = "unauthorized")]
    fn test_close_credit_line_unauthorized_closer() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);
        let other = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.close_credit_line(&borrower, &other);
    }

    #[test]
    fn test_draw_credit_updates_utilized() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);

        client.draw_credit(&borrower, &200_i128);
        assert_eq!(
            client.get_credit_line(&borrower).unwrap().utilized_amount,
            200
        );

        client.draw_credit(&borrower, &300_i128);
        assert_eq!(
            client.get_credit_line(&borrower).unwrap().utilized_amount,
            500
        );
    }

    // =========================================================================
    // Integration tests: full lifecycle flows (open → draw → repay → close)
    // =========================================================================

    /// End-to-end flow: init → open → draw × 2 → repay × 2 (stub) → admin close.
    ///
    /// Asserts every state transition and event count along the way.
    /// Events are checked immediately after each emitting call (before any
    /// subsequent contract call clears the per-invocation event buffer).
    ///
    /// NOTE: `repay_credit` is currently a stub that validates active state but
    /// does not reduce `utilized_amount`. Admin force-close is therefore used at
    /// the end. The borrower-self-close variant is covered by
    /// `test_integration_flow_borrower_close_zero_utilized`.
    #[test]
    fn test_integration_flow_open_draw_repay_close() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        // --- 1. Init --------------------------------------------------------
        client.init(&admin);

        // --- 2. Open credit line --------------------------------------------
        client.open_credit_line(&borrower, &10_000_i128, &500_u32, &75_u32);
        // CreditLineOpened event — check BEFORE next contract call resets buffer
        assert_eq!(env.events().all().len(), 1);

        let cl = client.get_credit_line(&borrower).unwrap();
        assert_eq!(cl.borrower, borrower);
        assert_eq!(cl.credit_limit, 10_000);
        assert_eq!(cl.utilized_amount, 0);
        assert_eq!(cl.interest_rate_bps, 500);
        assert_eq!(cl.risk_score, 75);
        assert_eq!(cl.status, CreditStatus::Active);

        // --- 3. First draw: 3 000 -------------------------------------------
        client.draw_credit(&borrower, &3_000_i128);
        // draw emits no events
        assert_eq!(env.events().all().len(), 0);

        let cl = client.get_credit_line(&borrower).unwrap();
        assert_eq!(cl.utilized_amount, 3_000);
        assert_eq!(cl.status, CreditStatus::Active);

        // --- 4. Second draw: 2 000 (cumulative: 5 000) ----------------------
        client.draw_credit(&borrower, &2_000_i128);
        assert_eq!(env.events().all().len(), 0);

        let cl = client.get_credit_line(&borrower).unwrap();
        assert_eq!(cl.utilized_amount, 5_000);
        assert_eq!(cl.credit_limit, 10_000);
        assert_eq!(cl.status, CreditStatus::Active);

        // --- 5. Repay × 2 (stub: validates active state; utilized stays 5 000) ---
        client.repay_credit(&borrower, &2_500_i128);
        assert_eq!(env.events().all().len(), 0);

        let cl = client.get_credit_line(&borrower).unwrap();
        assert_eq!(cl.status, CreditStatus::Active);
        assert_eq!(cl.utilized_amount, 5_000); // stub — not yet reduced

        client.repay_credit(&borrower, &2_500_i128);
        assert_eq!(env.events().all().len(), 0);

        let cl = client.get_credit_line(&borrower).unwrap();
        assert_eq!(cl.status, CreditStatus::Active);
        assert_eq!(cl.utilized_amount, 5_000); // stub — not yet reduced

        // --- 6. Admin force-close -------------------------------------------
        client.close_credit_line(&borrower, &admin);
        // CreditLineClosed event — check BEFORE next contract call resets buffer
        assert_eq!(env.events().all().len(), 1);

        let cl = client.get_credit_line(&borrower).unwrap();
        assert_eq!(cl.status, CreditStatus::Closed);
        assert_eq!(cl.credit_limit, 10_000);
        assert_eq!(cl.interest_rate_bps, 500);
        assert_eq!(cl.risk_score, 75);
    }

    /// Integration variant: open → (no draw) → borrower self-closes when utilized == 0.
    ///
    /// Confirms a borrower may close their own line with no outstanding balance,
    /// and that the correct state and events are recorded.
    #[test]
    fn test_integration_flow_borrower_close_zero_utilized() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        // --- 1. Init & Open -------------------------------------------------
        client.init(&admin);
        client.open_credit_line(&borrower, &5_000_i128, &300_u32, &60_u32);
        // CreditLineOpened event — check BEFORE next contract call resets buffer
        assert_eq!(env.events().all().len(), 1);

        let cl = client.get_credit_line(&borrower).unwrap();
        assert_eq!(cl.status, CreditStatus::Active);
        assert_eq!(cl.utilized_amount, 0);
        assert_eq!(cl.credit_limit, 5_000);
        assert_eq!(cl.interest_rate_bps, 300);
        assert_eq!(cl.risk_score, 60);

        // --- 2. Borrower closes with zero utilization -----------------------
        client.close_credit_line(&borrower, &borrower);
        // CreditLineClosed event — check BEFORE next contract call resets buffer
        assert_eq!(env.events().all().len(), 1);

        let cl = client.get_credit_line(&borrower).unwrap();
        assert_eq!(cl.status, CreditStatus::Closed);
        assert_eq!(cl.utilized_amount, 0);
    }
}
