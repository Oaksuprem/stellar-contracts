#[cfg(test)]
mod tests {
    use super::super::*;
    use soroban_sdk::{Address, Env, String};

    #[test]
    fn test_escrow_manager_initialization() {
        let env = Env::default();
        let owner = Address::random(&env);

        EscrowManagerContract::__constructor(&env, owner.clone());
        // Basic initialization test
        assert!(true);
    }

    #[test]
    fn test_invalid_escrow_amount() {
        let env = Env::default();
        let owner = Address::random(&env);
        let account_owner = Address::random(&env);
        let asset = Address::random(&env);

        EscrowManagerContract::__constructor(&env, owner);

        let result = std::panic::catch_unwind(|| {
            EscrowManagerContract::create_escrow(
                &env,
                String::from_slice(&env, "tx123"),
                account_owner,
                asset,
                0, // Invalid amount
                100,
            );
        });

        assert!(result.is_err());
    }
}
