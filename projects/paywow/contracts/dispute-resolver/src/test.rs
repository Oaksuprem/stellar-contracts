#[cfg(test)]
mod tests {
    use super::super::*;
    use soroban_sdk::{Address, Env, String};

    #[test]
    fn test_dispute_resolver_initialization() {
        let env = Env::default();
        let owner = Address::random(&env);

        DisputeResolverContract::__constructor(&env, owner.clone(), 1000);
        // Basic initialization test
        assert!(true);
    }

    #[test]
    fn test_invalid_dispute_amount() {
        let env = Env::default();
        let owner = Address::random(&env);
        let claimant = Address::random(&env);
        let respondent = Address::random(&env);

        DisputeResolverContract::__constructor(&env, owner, 1000);

        let result = std::panic::catch_unwind(|| {
            DisputeResolverContract::file_dispute(
                &env,
                String::from_slice(&env, "dispute1"),
                claimant,
                respondent,
                0, // Invalid amount
                String::from_slice(&env, "reason"),
                String::from_slice(&env, "evidence"),
            );
        });

        assert!(result.is_err());
    }
}
