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

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
}

#[contract]
pub struct Credit;

fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("Admin not initialized")
}

fn require_admin(env: &Env) {
    let admin = get_admin(env);
    admin.require_auth();
}

fn load_credit_line(env: &Env, borrower: &Address) -> CreditLineData {
    env.storage()
        .persistent()
        .get(borrower)
        .expect("Credit line not found")
}

fn save_credit_line(env: &Env, borrower: &Address, credit_line: &CreditLineData) {
    env.storage().persistent().set(borrower, credit_line);
}

fn publish_credit_event(
    env: &Env,
    event_type: Symbol,
    borrower: &Address,
    status: CreditStatus,
    credit_limit: i128,
    interest_rate_bps: u32,
    risk_score: u32,
) {
    env.events().publish(
        (symbol_short!("credit"), event_type.clone()),
        CreditLineEvent {
            event_type,
            borrower: borrower.clone(),
            status,
            credit_limit,
            interest_rate_bps,
            risk_score,
        },
    );
}

#[contractimpl]
impl Credit {
    /// @notice Initialize the contract and store the admin address.
    /// @param admin The protocol admin who can perform privileged actions.
    pub fn init(env: Env, admin: Address) -> () {
        env.storage().instance().set(&DataKey::Admin, &admin);
        ()
    }

    /// @notice Open a new credit line for a borrower.
    /// @dev This is currently unrestricted to keep parity with the existing stub behavior.
    /// @param borrower Borrower address.
    /// @param credit_limit Maximum principal borrowable.
    /// @param interest_rate_bps Interest rate in basis points.
    /// @param risk_score Risk score assigned by underwriting.
    /// @return Unit.
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

        save_credit_line(&env, &borrower, &credit_line);
        publish_credit_event(
            &env,
            symbol_short!("opened"),
            &borrower,
            CreditStatus::Active,
            credit_limit,
            interest_rate_bps,
            risk_score,
        );
        ()
    }

    /// @notice Draw principal from an active credit line.
    /// @dev Reverts if borrower is not authenticated, line does not exist,
    ///      amount is not positive, line is not Active, or limit would be exceeded.
    /// @param borrower Borrower address.
    /// @param amount Amount to draw.
    pub fn draw_credit(env: Env, borrower: Address, amount: i128) -> () {
        borrower.require_auth();
        if amount <= 0 {
            panic!("Draw amount must be positive");
        }

        let mut credit_line = load_credit_line(&env, &borrower);
        if credit_line.status != CreditStatus::Active {
            panic!("Credit line is not active");
        }

        let new_utilized_amount = credit_line
            .utilized_amount
            .checked_add(amount)
            .expect("Utilized amount overflow");
        if new_utilized_amount > credit_line.credit_limit {
            panic!("Credit limit exceeded");
        }

        credit_line.utilized_amount = new_utilized_amount;
        save_credit_line(&env, &borrower, &credit_line);
        ()
    }

    /// @notice Repay credit from borrower.
    /// @dev Repayments are intentionally allowed even when a line is suspended.
    pub fn repay_credit(_env: Env, _borrower: Address, _amount: i128) -> () {
        // TODO: accept token, reduce utilized_amount, accrue interest
        ()
    }

    /// @notice Update risk parameters for a borrower.
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

    /// @notice Suspend a borrower's credit line.
    /// @dev Admin only. Only Active credit lines can be suspended.
    ///      A suspended line cannot draw new credit but can still be repaid.
    /// @param borrower Borrower address whose line will be suspended.
    pub fn suspend_credit_line(env: Env, borrower: Address) -> () {
        require_admin(&env);
        let mut credit_line = load_credit_line(&env, &borrower);
        if credit_line.status != CreditStatus::Active {
            panic!("Only active credit lines can be suspended");
        }

        credit_line.status = CreditStatus::Suspended;
        save_credit_line(&env, &borrower, &credit_line);
        publish_credit_event(
            &env,
            symbol_short!("suspend"),
            &borrower,
            CreditStatus::Suspended,
            credit_line.credit_limit,
            credit_line.interest_rate_bps,
            credit_line.risk_score,
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
    pub fn close_credit_line(env: Env, borrower: Address) -> () {
        let mut credit_line = load_credit_line(&env, &borrower);

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
        save_credit_line(&env, &borrower, &credit_line);
        publish_credit_event(
            &env,
            symbol_short!("closed"),
            &borrower,
            CreditStatus::Closed,
            credit_line.credit_limit,
            credit_line.interest_rate_bps,
            credit_line.risk_score,
        );
        ()
    }

    /// Mark a credit line as defaulted (admin).
    /// Emits a CreditLineDefaulted event.
    pub fn default_credit_line(env: Env, borrower: Address) -> () {
        let mut credit_line = load_credit_line(&env, &borrower);

        credit_line.status = CreditStatus::Defaulted;
        save_credit_line(&env, &borrower, &credit_line);
        publish_credit_event(
            &env,
            symbol_short!("default"),
            &borrower,
            CreditStatus::Defaulted,
            credit_line.credit_limit,
            credit_line.interest_rate_bps,
            credit_line.risk_score,
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
    #[should_panic(expected = "Only active credit lines can be suspended")]
    fn test_suspend_credit_line_only_when_active() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.suspend_credit_line(&borrower);
        client.suspend_credit_line(&borrower);
    }

    #[test]
    #[should_panic(expected = "Error(Auth, InvalidAction)")]
    fn test_suspend_credit_line_rejects_non_admin() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.suspend_credit_line(&borrower);
    }

    #[test]
    #[should_panic(expected = "Credit line is not active")]
    fn test_draw_rejected_when_suspended() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.suspend_credit_line(&borrower);
        client.draw_credit(&borrower, &1_i128);
    }

    #[test]
    fn test_draw_active_updates_utilization() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);

        client.draw_credit(&borrower, &400_i128);
        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.utilized_amount, 400_i128);

        client.draw_credit(&borrower, &600_i128);
        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.utilized_amount, 1000_i128);
    }

    #[test]
    #[should_panic(expected = "Credit limit exceeded")]
    fn test_draw_rejected_when_credit_limit_exceeded() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.draw_credit(&borrower, &1001_i128);
    }

    #[test]
    #[should_panic(expected = "Draw amount must be positive")]
    fn test_draw_rejected_for_non_positive_amount() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.draw_credit(&borrower, &0_i128);
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
        env.mock_all_auths();
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

    #[test]
    #[should_panic(expected = "Credit line not found")]
    fn test_draw_nonexistent_credit_line() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.draw_credit(&borrower, &1_i128);
    }
}
