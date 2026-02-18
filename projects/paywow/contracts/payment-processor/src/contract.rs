//! Payment Processor Contract
//!
//! Core payment processing contract that handles fee-based transactions.
//! Integrates with OpenZeppelin's fee abstraction and fungible token contracts.

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, Env, Map,
    Symbol, String, IntoVal, TryFromVal,
};
use stellar_tokens::fungible::FungibleToken;
use stellar_access::ownable::Ownable;

// Storage keys
pub const OWNER: Symbol = symbol_short!("OWNER");
pub const PAYMENT_TOKEN: Symbol = symbol_short!("PTOKEN");
pub const PLATFORM_FEE: Symbol = symbol_short!("PLFEE");
pub const MERCHANT_FEES: Symbol = symbol_short!("MFEES");
pub const COLLECTED_FEES: Symbol = symbol_short!("CFEES");
pub const WHITELIST: Symbol = symbol_short!("WHITELIST");

#[contract]
pub struct PaymentProcessorContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PaymentError {
    Unauthorized = 1,
    InvalidAmount = 2,
    TokenNotSupported = 3,
    FeeExceedsPayment = 4,
    InsufficientBalance = 5,
    TransferFailed = 6,
    InvalidFeePercentage = 7,
}

/// Payment event
#[derive(Clone, IntoVal, TryFromVal)]
#[soroban_sdk::contracttype]
pub struct PaymentProcessed {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
    pub platform_fee: i128,
    pub merchant_fee: i128,
    pub payment_id: String,
}

#[contractimpl]
impl PaymentProcessorContract {
    /// Initialize the payment processor
    ///
    /// # Arguments
    /// * `owner` - Admin address for the payment processor
    /// * `payment_token` - Token address to use for payments
    /// * `platform_fee_bps` - Platform fee in basis points (e.g., 100 = 1%)
    pub fn __constructor(
        e: &Env,
        owner: Address,
        payment_token: Address,
        platform_fee_bps: u32,
    ) {
        if platform_fee_bps > 10000 {
            panic_with_error!(e, PaymentError::InvalidFeePercentage);
        }

        e.storage().instance().set(&OWNER, &owner);
        e.storage().instance().set(&PAYMENT_TOKEN, &payment_token);
        e.storage().instance().set(&PLATFORM_FEE, &platform_fee_bps);

        let whitelist: Map<Address, bool> = Map::new(e);
        e.storage().instance().set(&WHITELIST, &whitelist);
    }

    /// Get the current platform fee in basis points
    pub fn get_platform_fee(e: &Env) -> u32 {
        e.storage()
            .instance()
            .get::<_, u32>(&PLATFORM_FEE)
            .expect("platform fee not set")
    }

    /// Get the payment token address
    pub fn get_payment_token(e: &Env) -> Address {
        e.storage()
            .instance()
            .get::<_, Address>(&PAYMENT_TOKEN)
            .expect("payment token not set")
    }

    /// Process a payment from one account to another
    ///
    /// # Arguments
    /// * `from` - Sender address
    /// * `to` - Recipient address
    /// * `amount` - Amount to send (before fees)
    /// * `merchant` - Merchant address (optional, for merchant fees)
    /// * `merchant_fee_bps` - Merchant fee in basis points
    /// * `payment_id` - Unique payment identifier
    pub fn process_payment(
        e: &Env,
        from: Address,
        to: Address,
        amount: i128,
        merchant: Address,
        merchant_fee_bps: u32,
        payment_id: String,
    ) {
        from.require_auth();

        if amount <= 0 {
            panic_with_error!(e, PaymentError::InvalidAmount);
        }

        let platform_fee_bps = Self::get_platform_fee(e);
        let payment_token = Self::get_payment_token(e);

        // Calculate fees
        let platform_fee = (amount as u128 * platform_fee_bps as u128 / 10000) as i128;
        let merchant_fee = (amount as u128 * merchant_fee_bps as u128 / 10000) as i128;
        let total_fee = platform_fee + merchant_fee;

        if total_fee >= amount {
            panic_with_error!(e, PaymentError::FeeExceedsPayment);
        }

        // Transfer amount to recipient
        Self::transfer_token(e, &payment_token, &from, &to, amount);

        // Transfer platform fee to owner
        Self::transfer_token(e, &payment_token, &from, &Self::get_owner(e), platform_fee);

        // Transfer merchant fee to merchant
        if merchant_fee > 0 {
            Self::transfer_token(e, &payment_token, &from, &merchant, merchant_fee);
        }

        // Update collected fees
        let mut collected: i128 = e
            .storage()
            .instance()
            .get::<_, i128>(&COLLECTED_FEES)
            .unwrap_or(0);
        collected += platform_fee;
        e.storage()
            .instance()
            .set(&COLLECTED_FEES, &collected);

        // Emit event
        e.events().publish(
            (symbol_short!("PAYMENT"),),
            PaymentProcessed {
                from: from.clone(),
                to,
                amount,
                platform_fee,
                merchant_fee,
                payment_id,
            },
        );
    }

    /// Transfer tokens using the fungible token contract
    fn transfer_token(
        e: &Env,
        token: &Address,
        from: &Address,
        to: &Address,
        amount: i128,
    ) {
        let token_client = TokenClient::new(e, token);
        if let Err(_) = token_client.transfer(from, to, &amount) {
            panic_with_error!(e, PaymentError::TransferFailed);
        }
    }

    /// Get the owner of this contract
    fn get_owner(e: &Env) -> Address {
        e.storage()
            .instance()
            .get::<_, Address>(&OWNER)
            .expect("owner not set")
    }

    /// Get total collected platform fees
    pub fn get_collected_fees(e: &Env) -> i128 {
        e.storage()
            .instance()
            .get::<_, i128>(&COLLECTED_FEES)
            .unwrap_or(0)
    }

    /// Withdraw collected fees
    #[only_owner]
    pub fn withdraw_fees(e: &Env, amount: i128) {
        let owner = Self::get_owner(e);
        let token = Self::get_payment_token(e);
        let collected = Self::get_collected_fees(e);

        if amount > collected {
            panic_with_error!(e, PaymentError::InvalidAmount);
        }

        Self::transfer_token(e, &token, &Self::get_owner(e), &owner, amount);

        let new_collected = collected - amount;
        e.storage()
            .instance()
            .set(&COLLECTED_FEES, &new_collected);
    }
}

/// Client for the payment processor contract to be used by other contracts
pub struct PaymentProcessorClient<'a> {
    pub env: &'a Env,
    pub address: &'a Address,
}

impl<'a> PaymentProcessorClient<'a> {
    pub fn new(env: &'a Env, address: &'a Address) -> Self {
        Self { env, address }
    }

    pub fn process_payment(
        &self,
        from: &Address,
        to: &Address,
        amount: i128,
        merchant: &Address,
        merchant_fee_bps: u32,
        payment_id: String,
    ) {
        let args = (
            symbol_short!("process_payment"),
            from,
            to,
            amount,
            merchant,
            merchant_fee_bps,
            payment_id,
        );
        let _: () = self
            .env
            .invoke_contract(&self.address, &args.0, (&args.1, &args.2, &args.3, &args.4, &args.5, &args.6));
    }

    pub fn get_platform_fee(&self) -> u32 {
        let args = (symbol_short!("plat_fee"),);
        self.env
            .invoke_contract(&self.address, &args.0, &())
    }
}

/// Token client for fungible token operations
pub struct TokenClient<'a> {
    pub env: &'a Env,
    pub address: &'a Address,
}

impl<'a> TokenClient<'a> {
    pub fn new(env: &'a Env, address: &'a Address) -> Self {
        Self { env, address }
    }

    pub fn transfer(&self, from: &Address, to: &Address, amount: &i128) -> Result<(), PaymentError> {
        let args = (symbol_short!("transfer"), from, to, amount);
        match self.env.invoke_contract::<()>(&self.address, &args.0, &(args.1, args.2, args.3)) {
            _ => Ok(()),
        }
    }

    pub fn balance(&self, account: &Address) -> i128 {
        let args = (symbol_short!("balance"), account);
        self.env
            .invoke_contract(&self.address, &args.0, &args.1)
    }
}
