#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, panic_with_error};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreditStatus {
    Active = 0,
    Suspended = 1,
    Defaulted = 2,
    Closed = 3,
}

#[contracttype]
#[derive(Debug, Clone, PartialEq)]
pub enum CreditError {
    CreditLineNotFound = 1,
    InvalidCreditStatus = 2,
    InvalidAmount = 3,
    InsufficientUtilization = 4,
    Unauthorized = 5,
}

impl Into<soroban_sdk::Error> for CreditError {
    fn into(self) -> soroban_sdk::Error {
        soroban_sdk::Error::from_contract_error(self as u32)
    }
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
        env: Env,
        borrower: Address,
        credit_limit: i128,
        interest_rate_bps: u32,
        risk_score: u32,
    ) -> () {
        let credit_data = CreditLineData {
            borrower: borrower.clone(),
            credit_limit,
            utilized_amount: 0_i128,
            interest_rate_bps,
            risk_score,
            status: CreditStatus::Active,
        };
        
        let credit_key = (Symbol::new(&env, "CREDIT_LINE"), borrower.clone());
        env.storage().persistent().set(&credit_key, &credit_data);
        
        // Emit credit line opened event
        env.events().publish(
            (Symbol::new(&env, "credit_opened"), borrower),
            (credit_limit, interest_rate_bps, risk_score)
        );
    }

    /// Draw from credit line (borrower).
    pub fn draw_credit(env: Env, borrower: Address, amount: i128) -> () {
        if amount <= 0 {
            panic_with_error!(&env, CreditError::InvalidAmount);
        }

        let credit_key = (Symbol::new(&env, "CREDIT_LINE"), borrower.clone());
        let mut credit_data: CreditLineData = env.storage().persistent().get(&credit_key)
            .unwrap_or_else(|| panic_with_error!(&env, CreditError::CreditLineNotFound));

        if credit_data.status != CreditStatus::Active {
            panic_with_error!(&env, CreditError::InvalidCreditStatus);
        }

        let available_credit = credit_data.credit_limit.checked_sub(credit_data.utilized_amount)
            .expect("Credit limit should be >= utilized amount");
        
        if amount > available_credit {
            panic_with_error!(&env, CreditError::InsufficientUtilization);
        }

        credit_data.utilized_amount = credit_data.utilized_amount.checked_add(amount)
            .expect("Utilized amount should not overflow credit limit");

        env.storage().persistent().set(&credit_key, &credit_data);

        // Emit draw event
        env.events().publish(
            (Symbol::new(&env, "draw"), borrower.clone()),
            (amount, credit_data.utilized_amount)
        );
    }

    /// Repay credit (borrower).
    /// 
    /// Repays the specified amount from the borrower's credit line.
    /// The amount is applied to reduce the utilized_amount, with any excess
    /// amount ignored (no refund for overpayment).
    /// 
    /// # Arguments
    /// * `borrower` - The address of the borrower making the repayment
    /// * `amount` - The repayment amount (must be > 0)
    /// 
    /// # Errors
    /// * `CreditLineNotFound` - If no credit line exists for the borrower
    /// * `InvalidCreditStatus` - If credit line is not Active or Suspended
    /// * `InvalidAmount` - If amount <= 0
    /// 
    /// # Events
    /// Emits a repayment event with borrower address and amount applied
    pub fn repay_credit(env: Env, borrower: Address, amount: i128) -> () {
        // Validate input
        if amount <= 0 {
            panic_with_error!(&env, CreditError::InvalidAmount);
        }

        // Get credit line data
        let credit_key = (Symbol::new(&env, "CREDIT_LINE"), borrower.clone());
        let mut credit_data: CreditLineData = env.storage().persistent().get(&credit_key)
            .unwrap_or_else(|| panic_with_error!(&env, CreditError::CreditLineNotFound));

        // Validate credit status
        if credit_data.status != CreditStatus::Active && credit_data.status != CreditStatus::Suspended {
            panic_with_error!(&env, CreditError::InvalidCreditStatus);
        }

        // Calculate amount to apply (capped at current utilization)
        let amount_to_apply = if amount > credit_data.utilized_amount {
            credit_data.utilized_amount
        } else {
            amount
        };

        // Update utilized amount
        credit_data.utilized_amount = credit_data.utilized_amount.checked_sub(amount_to_apply)
            .expect("Underflow should not occur with proper validation");

        // Store updated credit line data
        env.storage().persistent().set(&credit_key, &credit_data);

        // Emit repayment event
        env.events().publish(
            (Symbol::new(&env, "repayment"), borrower.clone()),
            (amount_to_apply, credit_data.utilized_amount)
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
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Symbol;

    fn call_contract<F>(env: &Env, contract_id: &Address, f: F) 
    where F: FnOnce() {
        env.as_contract(contract_id, f);
    }

    fn setup_test(env: &Env) -> (Address, Address, Address) {
        let admin = Address::generate(env);
        let borrower = Address::generate(env);
        let contract_id = env.register(Credit, ());
        
        env.as_contract(&contract_id, || {
            Credit::init(env.clone(), admin.clone());
            Credit::open_credit_line(env.clone(), borrower.clone(), 1000_i128, 300_u32, 70_u32);
        });
        
        (admin, borrower, contract_id)
    }

    fn get_credit_data(env: &Env, contract_id: &Address, borrower: &Address) -> CreditLineData {
        let credit_key = (Symbol::new(env, "CREDIT_LINE"), borrower.clone());
        env.as_contract(contract_id, || {
            env.storage().persistent().get(&credit_key).unwrap()
        })
    }

    #[test]
    fn test_init_and_open_credit_line() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        let credit_data = get_credit_data(&env, &contract_id, &borrower);
        assert_eq!(credit_data.borrower, borrower);
        assert_eq!(credit_data.credit_limit, 1000_i128);
        assert_eq!(credit_data.utilized_amount, 0_i128);
        assert_eq!(credit_data.interest_rate_bps, 300_u32);
        assert_eq!(credit_data.risk_score, 70_u32);
        assert_eq!(credit_data.status, CreditStatus::Active);
        
        // Events are emitted - functionality verified through storage changes
    }

    #[test]
    fn test_draw_credit() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(), 500_i128);
        });
        
        let credit_data = get_credit_data(&env, &contract_id, &borrower);
        assert_eq!(credit_data.utilized_amount, 500_i128);
        
        // Events are emitted - functionality verified through storage changes
    }

    #[test]
    fn test_repay_credit_partial() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // First draw some credit
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(), 500_i128);
        });
        assert_eq!(get_credit_data(&env, &contract_id, &borrower).utilized_amount, 500_i128);
        
        // Partial repayment
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(), 200_i128);
        });
        
        let credit_data = get_credit_data(&env, &contract_id, &borrower);
        assert_eq!(credit_data.utilized_amount, 300_i128); // 500 - 200
    }

    #[test]
    fn test_repay_credit_full() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // Draw some credit
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(), 500_i128);
        });
        assert_eq!(get_credit_data(&env, &contract_id, &borrower).utilized_amount, 500_i128);
        
        // Full repayment
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(), 500_i128);
        });
        
        let credit_data = get_credit_data(&env, &contract_id, &borrower);
        assert_eq!(credit_data.utilized_amount, 0_i128); // Fully repaid
    }

    #[test]
    fn test_repay_credit_overpayment() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // Draw some credit
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(),300_i128);
        });
        assert_eq!(get_credit_data(&env, &contract_id, &borrower).utilized_amount, 300_i128);
        
        // Overpayment (pay more than utilized)
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),500_i128);
        });
        
        let credit_data = get_credit_data(&env, &contract_id, &borrower);
        assert_eq!(credit_data.utilized_amount, 0_i128); // Should be capped at 0
    }

    #[test]
    fn test_repay_credit_zero_utilization() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // Try to repay when no credit is utilized
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),100_i128);
        });
        
        let credit_data = get_credit_data(&env, &contract_id, &borrower);
        assert_eq!(credit_data.utilized_amount, 0_i128); // Should remain 0
        
    }

    #[test]
    fn test_repay_credit_suspended_status() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // Draw some credit
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(),500_i128);
        });
        
        // Manually set status to Suspended
        let credit_key = (Symbol::new(&env, "CREDIT_LINE"), borrower.clone());
        let mut credit_data = get_credit_data(&env, &contract_id, &borrower);
        credit_data.status = CreditStatus::Suspended;
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&credit_key, &credit_data);
        });
        
        // Should be able to repay even when suspended
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),200_i128);
        });
        
        let updated_data = get_credit_data(&env, &contract_id, &borrower);
        assert_eq!(updated_data.utilized_amount, 300_i128);
        assert_eq!(updated_data.status, CreditStatus::Suspended); // Status should remain Suspended
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")]
    fn test_repay_credit_invalid_amount_zero() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),0_i128);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")]
    fn test_repay_credit_invalid_amount_negative() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),-100_i128);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_repay_credit_no_credit_line() {
        let env = Env::default();
        let (_admin, _borrower, contract_id) = setup_test(&env);
        let unknown_borrower = Address::generate(&env);
        
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), unknown_borrower, 100_i128);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_repay_credit_defaulted_status() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // Draw some credit
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(),500_i128);
        });
        
        // Manually set status to Defaulted
        let credit_key = (Symbol::new(&env, "CREDIT_LINE"), borrower.clone());
        let mut credit_data = get_credit_data(&env, &contract_id, &borrower);
        credit_data.status = CreditStatus::Defaulted;
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&credit_key, &credit_data);
        });
        
        // Should panic when trying to repay from Defaulted status
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),100_i128);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_repay_credit_closed_status() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // Draw some credit
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(),500_i128);
        });
        
        // Manually set status to Closed
        let credit_key = (Symbol::new(&env, "CREDIT_LINE"), borrower.clone());
        let mut credit_data = get_credit_data(&env, &contract_id, &borrower);
        credit_data.status = CreditStatus::Closed;
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&credit_key, &credit_data);
        });
        
        // Should panic when trying to repay from Closed status
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),100_i128);
        });
    }

    #[test]
    fn test_repay_credit_multiple_operations() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // Multiple draw and repay operations
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(),200_i128);
        });
        assert_eq!(get_credit_data(&env, &contract_id, &borrower).utilized_amount, 200_i128);
        
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),50_i128);
        });
        assert_eq!(get_credit_data(&env, &contract_id, &borrower).utilized_amount, 150_i128);
        
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(),300_i128);
        });
        assert_eq!(get_credit_data(&env, &contract_id, &borrower).utilized_amount, 450_i128);
        
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),450_i128);
        });
        assert_eq!(get_credit_data(&env, &contract_id, &borrower).utilized_amount, 0_i128);
        
        // Try to overpay after full repayment
        call_contract(&env, &contract_id, || {
            Credit::repay_credit(env.clone(), borrower.clone(),100_i128);
        });
        assert_eq!(get_credit_data(&env, &contract_id, &borrower).utilized_amount, 0_i128);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_draw_credit_insufficient_available() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // Try to draw more than credit limit - should panic
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(),1500_i128);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")]
    fn test_draw_credit_invalid_amount() {
        let env = Env::default();
        let (_admin, borrower, contract_id) = setup_test(&env);
        
        // Try to draw zero amount - should panic
        call_contract(&env, &contract_id, || {
            Credit::draw_credit(env.clone(), borrower.clone(),0_i128);
        });
    }
}
