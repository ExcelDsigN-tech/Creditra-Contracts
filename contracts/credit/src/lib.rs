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

    /// @notice Draw from an active credit line for the borrower.
    /// @dev Reverts if amount is non-positive, line not active, limit exceeded, or arithmetic overflow.
    /// @param env Soroban environment.
    /// @param borrower Address whose credit line will be debited.
    /// @param amount Amount to draw, in protocol units.
    pub fn draw_credit(env: Env, borrower: Address, amount: i128) -> () {
        if amount <= 0 {
            panic!("draw amount must be positive");
        }

        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        if credit_line.status != CreditStatus::Active {
            panic!("credit line not active");
        }

        let new_utilized = credit_line
            .utilized_amount
            .checked_add(amount)
            .expect("utilized_amount overflow");

        if new_utilized > credit_line.credit_limit {
            panic!("draw exceeds credit limit");
        }

        credit_line.utilized_amount = new_utilized;

        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

        ()
    }

    /// @notice Repay part of an existing credit line.
    /// @dev Reverts if amount is non-positive, exceeds utilized amount, line is closed, or arithmetic underflow occurs.
    /// @param env Soroban environment.
    /// @param borrower Address whose credit line will be repaid.
    /// @param amount Amount to repay, in protocol units.
    pub fn repay_credit(env: Env, borrower: Address, amount: i128) -> () {
        if amount <= 0 {
            panic!("repay amount must be positive");
        }

        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        if credit_line.status == CreditStatus::Closed {
            panic!("cannot repay closed credit line");
        }

        if amount > credit_line.utilized_amount {
            panic!("repay exceeds utilized amount");
        }

        let new_utilized = credit_line
            .utilized_amount
            .checked_sub(amount)
            .expect("utilized_amount underflow");

        credit_line.utilized_amount = new_utilized;

        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

        ()
    }

    /// Update risk parameters (admin/risk engine).
    /// @notice Update credit limit and risk parameters for a borrower.
    /// @dev Reverts if the new limit is below the current utilized amount.
    /// @param env Soroban environment.
    /// @param borrower Address whose credit line will be updated.
    /// @param credit_limit New total credit limit.
    /// @param interest_rate_bps New interest rate, in basis points.
    /// @param risk_score New risk score for the borrower.
    pub fn update_risk_parameters(
        env: Env,
        borrower: Address,
        credit_limit: i128,
        interest_rate_bps: u32,
        risk_score: u32,
    ) -> () {
        let mut credit_line: CreditLineData = env
            .storage()
            .persistent()
            .get(&borrower)
            .expect("Credit line not found");

        if credit_limit < credit_line.utilized_amount {
            panic!("new credit limit below utilized amount");
        }

        credit_line.credit_limit = credit_limit;
        credit_line.interest_rate_bps = interest_rate_bps;
        credit_line.risk_score = risk_score;

        env.storage()
            .persistent()
            .set(&borrower, &credit_line);

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

    #[test]
    fn test_draw_credit_within_limit_uses_checked_add() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);

        let large_limit = i128::MAX - 10;
        client.open_credit_line(&borrower, &large_limit, &300_u32, &70_u32);

        client.draw_credit(&borrower, &5_i128);

        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.utilized_amount, 5);
        assert_eq!(credit_line.credit_limit, large_limit);
    }

    #[test]
    #[should_panic(expected = "utilized_amount overflow")]
    fn test_draw_credit_overflow_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &i128::MAX, &300_u32, &70_u32);

        env.as_contract(&contract_id, || {
            let mut credit_line: CreditLineData = env
                .storage()
                .persistent()
                .get(&borrower)
                .expect("Credit line not found");
            credit_line.utilized_amount = i128::MAX;
            env.storage()
                .persistent()
                .set(&borrower, &credit_line);
        });

        client.draw_credit(&borrower, &1_i128);
    }

    #[test]
    #[should_panic(expected = "draw exceeds credit limit")]
    fn test_draw_credit_exceeding_limit_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &100_i128, &300_u32, &70_u32);

        client.draw_credit(&borrower, &200_i128);
    }

    #[test]
    #[should_panic(expected = "credit line not active")]
    fn test_draw_credit_rejects_inactive_status() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.suspend_credit_line(&borrower);

        client.draw_credit(&borrower, &10_i128);
    }

    #[test]
    fn test_repay_credit_uses_checked_sub() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);

        client.draw_credit(&borrower, &500_i128);
        client.repay_credit(&borrower, &200_i128);

        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.utilized_amount, 300);
    }

    #[test]
    #[should_panic(expected = "repay exceeds utilized amount")]
    fn test_repay_credit_exceeding_utilized_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.draw_credit(&borrower, &100_i128);

        client.repay_credit(&borrower, &200_i128);
    }

    #[test]
    #[should_panic(expected = "cannot repay closed credit line")]
    fn test_repay_credit_rejects_closed_line() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.close_credit_line(&borrower);

        client.repay_credit(&borrower, &10_i128);
    }

    #[test]
    #[should_panic(expected = "new credit limit below utilized amount")]
    fn test_update_risk_parameters_limit_below_utilized_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.draw_credit(&borrower, &800_i128);

        client.update_risk_parameters(&borrower, &500_i128, &300_u32, &70_u32);
    }

    #[test]
    fn test_update_risk_parameters_succeeds_and_preserves_utilization() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let borrower = Address::generate(&env);

        let contract_id = env.register(Credit, ());
        let client = CreditClient::new(&env, &contract_id);

        client.init(&admin);
        client.open_credit_line(&borrower, &1000_i128, &300_u32, &70_u32);
        client.draw_credit(&borrower, &400_i128);

        client.update_risk_parameters(&borrower, &1500_i128, &350_u32, &80_u32);

        let credit_line = client.get_credit_line(&borrower).unwrap();
        assert_eq!(credit_line.credit_limit, 1500);
        assert_eq!(credit_line.utilized_amount, 400);
        assert_eq!(credit_line.interest_rate_bps, 350);
        assert_eq!(credit_line.risk_score, 80);
    }
}
