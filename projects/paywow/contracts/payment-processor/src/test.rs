#[cfg(test)]
mod tests {
    use super::super::*;
    use soroban_sdk::{testutils::Address as TestAddress, Address, Env, String};

    #[test]
    fn test_payment_processor_initialization() {
        let env = Env::default();
        let contract = PaymentProcessorContract;
        let owner = Address::random(&env);
        let token = Address::random(&env);

        contract.__constructor(&env, owner.clone(), token.clone(), 100);

        assert_eq!(PaymentProcessorContract::get_platform_fee(&env), 100);
        assert_eq!(PaymentProcessorContract::get_payment_token(&env), token);
    }

    #[test]
    fn test_invalid_fee_percentage() {
        let env = Env::default();
        let contract = PaymentProcessorContract;
        let owner = Address::random(&env);
        let token = Address::random(&env);

        // Should panic with InvalidFeePercentage
        let result = std::panic::catch_unwind(|| {
            contract.__constructor(&env, owner.clone(), token.clone(), 10001);
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_collected_fees() {
        let env = Env::default();
        let contract = PaymentProcessorContract;
        let owner = Address::random(&env);
        let token = Address::random(&env);

        contract.__constructor(&env, owner.clone(), token.clone(), 100);

        assert_eq!(PaymentProcessorContract::get_collected_fees(&env), 0);
    }
}
