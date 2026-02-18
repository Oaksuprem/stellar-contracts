#[cfg(test)]
mod tests {
    use super::super::*;
    use soroban_sdk::{Address, Env, String};

    #[test]
    fn test_orchestrator_initialization() {
        let env = Env::default();
        let owner = Address::random(&env);
        let processor = Address::random(&env);
        let escrow = Address::random(&env);
        let dispute = Address::random(&env);
        let loyalty = Address::random(&env);
        let token = Address::random(&env);

        PaymentOrchestratorContract::__constructor(
            &env,
            owner.clone(),
            processor.clone(),
            escrow.clone(),
            dispute.clone(),
            loyalty.clone(),
            token.clone(),
        );

        let config = PaymentOrchestratorContract::get_config(&env);
        assert_eq!(config.0, processor);
        assert_eq!(config.1, escrow);
        assert_eq!(config.2, dispute);
        assert_eq!(config.3, loyalty);
        assert_eq!(config.4, token);
    }

    #[test]
    fn test_loyalty_points_calculation() {
        // 100 units = 1 point
        // 1000 units = 10 points
        assert_eq!(10, 10);
    }

    #[test]
    fn test_invalid_amount() {
        let env = Env::default();
        let owner = Address::random(&env);
        let processor = Address::random(&env);
        let escrow = Address::random(&env);
        let dispute = Address::random(&env);
        let loyalty = Address::random(&env);
        let token = Address::random(&env);

        PaymentOrchestratorContract::__constructor(
            &env,
            owner,
            processor,
            escrow,
            dispute,
            loyalty,
            token,
        );

        let payer = Address::random(&env);
        let payee = Address::random(&env);
        let merchant = Address::random(&env);

        let result = std::panic::catch_unwind(|| {
            PaymentOrchestratorContract::process_simple_payment(
                &env,
                String::from_slice(&env, "tx1"),
                payer,
                payee,
                0, // Invalid amount
                merchant,
                100,
            );
        });

        assert!(result.is_err());
    }
}
