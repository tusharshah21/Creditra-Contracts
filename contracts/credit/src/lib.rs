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

/// Event emitted when a repayment is made
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepaymentEvent {
    pub borrower: Address,
    pub amount: i128,
    pub utilized_before: i128,
    pub utilized_after: i128,
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

        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

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
    pub fn draw_credit(env: Env, borrower: Address, amount: i128) -> () {
        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        // Check credit limit
        if credit_line.utilized_amount + amount > credit_line.credit_limit {
            panic!("Credit limit exceeded");
        }

        credit_line.utilized_amount += amount;
        
        env.storage()
            .persistent()
            .set(&borrower, &credit_line);
        ()
    }

    /// Repay credit (borrower).
    /// Emits a RepaymentMade event.
    pub fn repay_credit(env: Env, borrower: Address, amount: i128) -> () {
        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        let utilized_before = credit_line.utilized_amount;
        
        // Reduce utilized amount (prevent going negative)
        if amount > credit_line.utilized_amount {
            credit_line.utilized_amount = 0;
        } else {
            credit_line.utilized_amount -= amount;
        }
        
        let utilized_after = credit_line.utilized_amount;
        
        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

        // Emit RepaymentMade event
        env.events().publish(
            (symbol_short!("credit"), symbol_short!("repay")),
            RepaymentEvent {
                borrower: borrower.clone(),
                amount,
                utilized_before,
                utilized_after,
            },
        );
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
        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

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

    /// Close a credit line (admin or borrower when utilized is 0).
    /// Emits a CreditLineClosed event.
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

        // Emit CreditLineClosed event
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
        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

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
    use soroban_sdk::testutils::{Address as _, Events};
    use soroban_sdk::{TryFromVal, TryIntoVal};

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
        client.close_credit_line(&borrower);

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
        client.close_credit_line(&borrower);
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

    // ========== Event Emission Tests ==========

    #[test]
    fn test_event_open_credit_line() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);

        // Get the events
        let events = env.events().all();
        let event_vec: soroban_sdk::Vec<(Address, soroban_sdk::Vec<soroban_sdk::Val>, soroban_sdk::Val)> = events;
        assert!(event_vec.len() > 0);
        
        let (_contract, topics, data) = event_vec.last().unwrap();

        // Verify event topics
        assert_eq!(topics.len(), 2);
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap(),
            symbol_short!("credit")
        );
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
            symbol_short!("opened")
        );

        // Verify event data
        let event_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(event_data.event_type, symbol_short!("opened"));
        assert_eq!(event_data.borrower, borrower);
        assert_eq!(event_data.status, CreditStatus::Active);
        assert_eq!(event_data.credit_limit, 1000);
        assert_eq!(event_data.interest_rate_bps, 300);
        assert_eq!(event_data.risk_score, 70);
    }

    #[test]
    fn test_event_suspend_credit_line() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.suspend_credit_line(&borrower);

        // Get the events (last event is the suspend event)
        let events = env.events().all();
        let (_contract, topics, data) = events.last().unwrap();

        // Verify event topics
        assert_eq!(topics.len(), 2);
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap(),
            symbol_short!("credit")
        );
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
            symbol_short!("suspend")
        );

        // Verify event data
        let event_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(event_data.event_type, symbol_short!("suspend"));
        assert_eq!(event_data.borrower, borrower);
        assert_eq!(event_data.status, CreditStatus::Suspended);
        assert_eq!(event_data.credit_limit, 1000);
        assert_eq!(event_data.interest_rate_bps, 300);
        assert_eq!(event_data.risk_score, 70);
    }

    #[test]
    fn test_event_close_credit_line() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &2000_i128, &400_u32, &80_u32);
        client.close_credit_line(&borrower);

        // Get the events (last event is the close event)
        let events = env.events().all();
        let (_contract, topics, data) = events.last().unwrap();

        // Verify event topics
        assert_eq!(topics.len(), 2);
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap(),
            symbol_short!("credit")
        );
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
            symbol_short!("closed")
        );

        // Verify event data
        let event_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(event_data.event_type, symbol_short!("closed"));
        assert_eq!(event_data.borrower, borrower);
        assert_eq!(event_data.status, CreditStatus::Closed);
        assert_eq!(event_data.credit_limit, 2000);
        assert_eq!(event_data.interest_rate_bps, 400);
        assert_eq!(event_data.risk_score, 80);
    }

    #[test]
    fn test_event_default_credit_line() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &3000_i128, &500_u32, &90_u32);
        client.default_credit_line(&borrower);

        // Get the events (last event is the default event)
        let events = env.events().all();
        let (_contract, topics, data) = events.last().unwrap();

        // Verify event topics
        assert_eq!(topics.len(), 2);
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap(),
            symbol_short!("credit")
        );
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
            symbol_short!("default")
        );

        // Verify event data
        let event_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(event_data.event_type, symbol_short!("default"));
        assert_eq!(event_data.borrower, borrower);
        assert_eq!(event_data.status, CreditStatus::Defaulted);
        assert_eq!(event_data.credit_limit, 3000);
        assert_eq!(event_data.interest_rate_bps, 500);
        assert_eq!(event_data.risk_score, 90);
    }

    #[test]
    fn test_event_repay_credit() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &5000_i128, &300_u32, &70_u32);
        
        // Draw 1000 from credit line
        client.draw_credit(&borrower, &1000_i128);
        
        // Repay 400
        client.repay_credit(&borrower, &400_i128);

        // Get the events (last event is the repay event)
        let events = env.events().all();
        let (_contract, topics, data) = events.last().unwrap();

        // Verify event topics
        assert_eq!(topics.len(), 2);
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap(),
            symbol_short!("credit")
        );
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
            symbol_short!("repay")
        );

        // Verify event data
        let event_data: RepaymentEvent = data.try_into_val(&env).unwrap();
        assert_eq!(event_data.borrower, borrower);
        assert_eq!(event_data.amount, 400);
        assert_eq!(event_data.utilized_before, 1000);
        assert_eq!(event_data.utilized_after, 600);
    }

    #[test]
    fn test_event_repay_credit_full_amount() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &5000_i128, &300_u32, &70_u32);
        
        // Draw 2000 from credit line
        client.draw_credit(&borrower, &2000_i128);
        
        // Repay full amount
        client.repay_credit(&borrower, &2000_i128);

        // Get the events (last event is the repay event)
        let events = env.events().all();
        let (_contract, _topics, data) = events.last().unwrap();

        // Verify event data
        let event_data: RepaymentEvent = data.try_into_val(&env).unwrap();
        assert_eq!(event_data.borrower, borrower);
        assert_eq!(event_data.amount, 2000);
        assert_eq!(event_data.utilized_before, 2000);
        assert_eq!(event_data.utilized_after, 0);
    }

    #[test]
    fn test_event_repay_credit_overpayment() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &5000_i128, &300_u32, &70_u32);
        
        // Draw 500 from credit line
        client.draw_credit(&borrower, &500_i128);
        
        // Repay more than utilized (should set to 0)
        client.repay_credit(&borrower, &1000_i128);

        // Get the events (last event is the repay event)
        let events = env.events().all();
        let (_contract, _topics, data) = events.last().unwrap();

        // Verify event data
        let event_data: RepaymentEvent = data.try_into_val(&env).unwrap();
        assert_eq!(event_data.borrower, borrower);
        assert_eq!(event_data.amount, 1000);
        assert_eq!(event_data.utilized_before, 500);
        assert_eq!(event_data.utilized_after, 0);
    }

    #[test]
    fn test_event_lifecycle_sequence() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        
        // Open credit line and check event
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        let events = env.events().all();
        let (_c, topics, data) = events.last().unwrap();
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
            symbol_short!("opened")
        );
        let open_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(open_data.status, CreditStatus::Active);
        
        // Suspend credit line and check event
        client.suspend_credit_line(&borrower);
        let events = env.events().all();
        let (_c, topics, data) = events.last().unwrap();
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
            symbol_short!("suspend")
        );
        let suspend_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(suspend_data.status, CreditStatus::Suspended);
        
        // Close credit line and check event
        client.close_credit_line(&borrower);
        let events = env.events().all();
        let (_c, topics, data) = events.last().unwrap();
        assert_eq!(
            Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(),
            symbol_short!("closed")
        );
        let close_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(close_data.status, CreditStatus::Closed);
    }

    #[test]
    fn test_event_multiple_repayments() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &10000_i128, &300_u32, &70_u32);
        
        // Draw 5000
        client.draw_credit(&borrower, &5000_i128);
        
        // First repayment
        client.repay_credit(&borrower, &1000_i128);
        let events = env.events().all();
        let (_c, _topics, data) = events.last().unwrap();
        let repay1_data: RepaymentEvent = data.try_into_val(&env).unwrap();
        assert_eq!(repay1_data.amount, 1000);
        assert_eq!(repay1_data.utilized_before, 5000);
        assert_eq!(repay1_data.utilized_after, 4000);
        
        // Second repayment
        client.repay_credit(&borrower, &2000_i128);
        let events = env.events().all();
        let (_c, _topics, data) = events.last().unwrap();
        let repay2_data: RepaymentEvent = data.try_into_val(&env).unwrap();
        assert_eq!(repay2_data.amount, 2000);
        assert_eq!(repay2_data.utilized_before, 4000);
        assert_eq!(repay2_data.utilized_after, 2000);
        
        // Third repayment
        client.repay_credit(&borrower, &1500_i128);
        let events = env.events().all();
        let (_c, _topics, data) = events.last().unwrap();
        let repay3_data: RepaymentEvent = data.try_into_val(&env).unwrap();
        assert_eq!(repay3_data.amount, 1500);
        assert_eq!(repay3_data.utilized_before, 2000);
        assert_eq!(repay3_data.utilized_after, 500);
    }

    #[test]
    fn test_event_data_consistency_across_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        
        // Open with specific parameters
        let credit_limit = 7500_i128;
        let interest_rate = 450_u32;
        let risk_score = 85_u32;
        
        // Open and verify event data
        client.open_credit_line(&borrower, &credit_limit, &interest_rate, &risk_score);
        let events = env.events().all();
        let (_c, _topics, data) = events.last().unwrap();
        let open_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(open_data.credit_limit, credit_limit);
        assert_eq!(open_data.interest_rate_bps, interest_rate);
        assert_eq!(open_data.risk_score, risk_score);
        assert_eq!(open_data.status, CreditStatus::Active);
        
        // Suspend and verify event data
        client.suspend_credit_line(&borrower);
        let events = env.events().all();
        let (_c, _topics, data) = events.last().unwrap();
        let suspend_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(suspend_data.credit_limit, credit_limit);
        assert_eq!(suspend_data.interest_rate_bps, interest_rate);
        assert_eq!(suspend_data.risk_score, risk_score);
        assert_eq!(suspend_data.status, CreditStatus::Suspended);
        
        // Default and verify event data
        client.default_credit_line(&borrower);
        let events = env.events().all();
        let (_c, _topics, data) = events.last().unwrap();
        let default_data: CreditLineEvent = data.try_into_val(&env).unwrap();
        assert_eq!(default_data.credit_limit, credit_limit);
        assert_eq!(default_data.interest_rate_bps, interest_rate);
        assert_eq!(default_data.risk_score, risk_score);
        assert_eq!(default_data.status, CreditStatus::Defaulted);
    }
}
