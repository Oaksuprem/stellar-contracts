//! Loyalty Program Contract
//!
//! NFT-based loyalty rewards system for Paywow transactions.
//! Issues NFTs as loyalty rewards and tracks customer engagement.

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, Env, Map,
    Symbol, String, IntoVal, TryFromVal,
};

// Storage keys
pub const LOYALTY_OWNER: Symbol = symbol_short!("OWNER");
pub const LOYALTY_TIERS: Symbol = symbol_short!("TIERS");
pub const CUSTOMER_POINTS: Symbol = symbol_short!("POINTS");
pub const ISSUED_REWARDS: Symbol = symbol_short!("REWARDS");
pub const TOTAL_REWARDS: Symbol = symbol_short!("TOTAL");

#[contract]
pub struct LoyaltyProgramContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LoyaltyError {
    Unauthorized = 1,
    InvalidPoints = 2,
    NoRewardEarned = 3,
    InvalidTier = 4,
}

/// Loyalty tier
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoyaltyTier {
    Bronze,  // 0-999 points
    Silver,  // 1000-4999 points
    Gold,    // 5000-9999 points
    Platinum, // 10000+ points
}

/// Loyalty reward NFT metadata
#[derive(Clone, IntoVal, TryFromVal)]
#[soroban_sdk::contracttype]
pub struct LoyaltyReward {
    pub token_id: u32,
    pub owner: Address,
    pub points_earned: u32,
    pub tier: u32,
    pub transaction_id: String,
    pub issued_at: u64,
}

#[contractimpl]
impl LoyaltyProgramContract {
    /// Initialize the loyalty program
    pub fn __constructor(e: &Env, owner: Address) {
        e.storage().instance().set(&LOYALTY_OWNER, &owner);
        let customer_points: Map<Address, u32> = Map::new(e);
        let rewards: Map<u32, LoyaltyReward> = Map::new(e);
        e.storage().instance().set(&CUSTOMER_POINTS, &customer_points);
        e.storage().instance().set(&ISSUED_REWARDS, &rewards);
        e.storage().instance().set(&TOTAL_REWARDS, &0u32);
    }

    /// Award loyalty points for a transaction
    ///
    /// # Arguments
    /// * `customer` - Customer address
    /// * `transaction_id` - Transaction identifier
    /// * `points` - Points to award
    pub fn award_points(
        e: &Env,
        customer: Address,
        transaction_id: String,
        points: u32,
    ) -> u32 {
        let owner: Address = e
            .storage()
            .instance()
            .get(&LOYALTY_OWNER)
            .expect("owner not set");
        owner.require_auth();

        if points == 0 {
            panic_with_error!(e, LoyaltyError::InvalidPoints);
        }

        // Update customer points
        let mut customer_points: Map<Address, u32> = e
            .storage()
            .instance()
            .get(&CUSTOMER_POINTS)
            .unwrap_or(Map::new(e));

        let mut current_points = customer_points.get(customer.clone()).unwrap_or(0);
        current_points += points;

        customer_points.set(customer.clone(), current_points);
        e.storage()
            .instance()
            .set(&CUSTOMER_POINTS, &customer_points);

        // Issue reward NFT if points threshold met
        let tier = Self::get_tier(current_points);
        let token_id = Self::issue_reward_nft(
            e,
            customer.clone(),
            current_points,
            transaction_id,
            tier,
        );

        // Emit event
        e.events().publish(
            (symbol_short!("LOYALTY"),),
            (
                symbol_short!("pts_award"),
                customer,
                points,
                current_points,
            ),
        );

        token_id
    }

    /// Get customer loyalty tier based on points
    pub fn get_customer_tier(e: &Env, customer: Address) -> u32 {
        let customer_points: Map<Address, u32> = e
            .storage()
            .instance()
            .get(&CUSTOMER_POINTS)
            .unwrap_or(Map::new(e));

        let points = customer_points.get(customer).unwrap_or(0);
        Self::get_tier(points)
    }

    /// Get customer total points
    pub fn get_customer_points(e: &Env, customer: Address) -> u32 {
        let customer_points: Map<Address, u32> = e
            .storage()
            .instance()
            .get(&CUSTOMER_POINTS)
            .unwrap_or(Map::new(e));

        customer_points.get(customer).unwrap_or(0)
    }

    /// Get loyalty tier
    fn get_tier(points: u32) -> u32 {
        if points < 1000 {
            0 // Bronze
        } else if points < 5000 {
            1 // Silver
        } else if points < 10000 {
            2 // Gold
        } else {
            3 // Platinum
        }
    }

    /// Issue a loyalty reward NFT
    fn issue_reward_nft(
        e: &Env,
        owner: Address,
        points_earned: u32,
        transaction_id: String,
        tier: u32,
    ) -> u32 {
        let mut rewards: Map<u32, LoyaltyReward> = e
            .storage()
            .instance()
            .get(&ISSUED_REWARDS)
            .unwrap_or(Map::new(e));

        let mut total_rewards: u32 = e
            .storage()
            .instance()
            .get(&TOTAL_REWARDS)
            .unwrap_or(0);

        let token_id = total_rewards + 1;
        let reward = LoyaltyReward {
            token_id,
            owner: owner.clone(),
            points_earned,
            tier,
            transaction_id,
            issued_at: e.ledger().sequence() as u64,
        };

        rewards.set(token_id, reward);
        e.storage().instance().set(&ISSUED_REWARDS, &rewards);

        total_rewards += 1;
        e.storage()
            .instance()
            .set(&TOTAL_REWARDS, &total_rewards);

        // Emit event
        e.events().publish(
            (symbol_short!("LOYALTY"),),
            (
                symbol_short!("nft_issued"),
                owner,
                token_id,
                tier,
            ),
        );

        token_id
    }

    /// Get reward NFT details
    pub fn get_reward(e: &Env, token_id: u32) -> LoyaltyReward {
        let rewards: Map<u32, LoyaltyReward> = e
            .storage()
            .instance()
            .get(&ISSUED_REWARDS)
            .expect("no rewards found");

        rewards
            .get(token_id)
            .expect_err("reward not found")
    }

    /// Get total issued rewards
    pub fn get_total_rewards(e: &Env) -> u32 {
        e.storage()
            .instance()
            .get(&TOTAL_REWARDS)
            .unwrap_or(0)
    }

    /// Burn loyalty points (admin function for redemptions)
    #[only_owner]
    pub fn redeem_points(e: &Env, customer: Address, points_to_redeem: u32) {
        let mut customer_points: Map<Address, u32> = e
            .storage()
            .instance()
            .get(&CUSTOMER_POINTS)
            .expect("no customers found");

        let current_points = customer_points
            .get(customer.clone())
            .unwrap_or(0);

        if current_points < points_to_redeem {
            panic_with_error!(e, LoyaltyError::InvalidPoints);
        }

        let new_points = current_points - points_to_redeem;
        if new_points > 0 {
            customer_points.set(customer.clone(), new_points);
        } else {
            customer_points.remove(customer.clone());
        }

        e.storage()
            .instance()
            .set(&CUSTOMER_POINTS, &customer_points);

        // Emit event
        e.events().publish(
            (symbol_short!("LOYALTY"),),
            (
                symbol_short!("pts_redm"),
                customer,
                points_to_redeem,
                new_points,
            ),
        );
    }
}

/// Client for loyalty program contract
pub struct LoyaltyProgramClient<'a> {
    pub env: &'a Env,
    pub address: &'a Address,
}

impl<'a> LoyaltyProgramClient<'a> {
    pub fn new(env: &'a Env, address: &'a Address) -> Self {
        Self { env, address }
    }

    pub fn award_points(&self, customer: &Address, transaction_id: String, points: u32) -> u32 {
        let args = (symbol_short!("awd_pts"), customer, transaction_id, points);
        self.env
            .invoke_contract(&self.address, &args.0, &(&args.1, &args.2, &args.3))
    }

    pub fn get_customer_points(&self, customer: &Address) -> u32 {
        let args = (symbol_short!("cust_pts"), customer);
        self.env
            .invoke_contract(&self.address, &args.0, &(&args.1))
    }

    pub fn get_customer_tier(&self, customer: &Address) -> u32 {
        let args = (symbol_short!("cust_tier"), customer);
        self.env
            .invoke_contract(&self.address, &args.0, &(&args.1))
    }
}
