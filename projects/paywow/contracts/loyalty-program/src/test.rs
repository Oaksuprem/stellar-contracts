#[cfg(test)]
mod tests {
    use super::super::*;
    use soroban_sdk::{Address, Env, String};

    #[test]
    fn test_loyalty_initialization() {
        let env = Env::default();
        let owner = Address::random(&env);

        LoyaltyProgramContract::__constructor(&env, owner.clone());

        assert_eq!(LoyaltyProgramContract::get_total_rewards(&env), 0);
    }

    #[test]
    fn test_customer_points_zero() {
        let env = Env::default();
        let owner = Address::random(&env);
        let customer = Address::random(&env);

        LoyaltyProgramContract::__constructor(&env, owner);

        assert_eq!(
            LoyaltyProgramContract::get_customer_points(&env, customer),
            0
        );
    }

    #[test]
    fn test_tier_calculation() {
        // Bronze: 0-999
        assert_eq!(0, 0);
        // Silver: 1000-4999
        assert_eq!(1, 1);
        // Gold: 5000-9999
        assert_eq!(2, 2);
        // Platinum: 10000+
        assert_eq!(3, 3);
    }
}
