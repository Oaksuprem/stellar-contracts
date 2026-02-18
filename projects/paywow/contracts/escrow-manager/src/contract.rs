//! Escrow Manager Contract
//!
//! Manages secure fund holding for transactions using tokenized vaults.
//! Integrates with OpenZeppelin's vault and fungible token contracts.

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, Env, Map,
    Symbol, String, IntoVal, TryFromVal,
};
use stellar_tokens::fungible::FungibleToken;

// Storage keys
pub const ESCROW_OWNER: Symbol = symbol_short!("OWNER");
pub const ESCROW_ACCOUNTS: Symbol = symbol_short!("ESCROWS");
pub const ESCROW_ASSETS: Symbol = symbol_short!("ASSETS");
pub const ESCROW_LOCKED_UNTIL: Symbol = symbol_short!("LOCKED");

#[contract]
pub struct EscrowManagerContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    Unauthorized = 1,
    EscrowNotFound = 2,
    InvalidAmount = 3,
    AssetNotSupported = 4,
    FundsLocked = 5,
    InsufficientFunds = 6,
}

/// Escrow account details
#[derive(Clone, IntoVal, TryFromVal)]
#[soroban_sdk::contracttype]
pub struct EscrowAccount {
    pub owner: Address,
    pub balance: i128,
    pub asset: Address,
    pub locked_until: u64,
    pub transaction_id: String,
}

#[contractimpl]
impl EscrowManagerContract {
    /// Initialize the escrow manager
    pub fn __constructor(e: &Env, owner: Address) {
        e.storage().instance().set(&ESCROW_OWNER, &owner);
        let escrows: Map<String, EscrowAccount> = Map::new(e);
        e.storage().instance().set(&ESCROW_ACCOUNTS, &escrows);
    }

    /// Create a new escrow account
    ///
    /// # Arguments
    /// * `transaction_id` - Unique transaction identifier
    /// * `owner` - Account owner
    /// * `asset` - Token address for the escrow
    /// * `amount` - Initial amount to hold
    /// * `locked_until` - Ledger sequence until funds are locked
    pub fn create_escrow(
        e: &Env,
        transaction_id: String,
        owner: Address,
        asset: Address,
        amount: i128,
        locked_until: u64,
    ) {
        owner.require_auth();

        if amount <= 0 {
            panic_with_error!(e, EscrowError::InvalidAmount);
        }

        // Store escrow account
        let mut escrows: Map<String, EscrowAccount> = e
            .storage()
            .instance()
            .get(&ESCROW_ACCOUNTS)
            .unwrap_or(Map::new(e));

        let escrow = EscrowAccount {
            owner: owner.clone(),
            balance: amount,
            asset: asset.clone(),
            locked_until,
            transaction_id: transaction_id.clone(),
        };

        escrows.set(transaction_id.clone(), escrow);
        e.storage()
            .instance()
            .set(&ESCROW_ACCOUNTS, &escrows);

        // Emit event
        e.events().publish(
            (symbol_short!("ESCROW"),),
            (
                symbol_short!("created"),
                owner.clone(),
                amount,
                transaction_id,
            ),
        );
    }

    /// Release funds from escrow to recipient
    ///
    /// # Arguments
    /// * `transaction_id` - Escrow transaction ID
    /// * `recipient` - Address to receive the funds
    pub fn release_escrow(e: &Env, transaction_id: String, recipient: Address) {
        let mut escrows: Map<String, EscrowAccount> = e
            .storage()
            .instance()
            .get(&ESCROW_ACCOUNTS)
            .expect("no escrows found");

        let escrow = escrows
            .get(transaction_id.clone())
            .expect_err("escrow not found");

        let current_ledger = e.ledger().sequence() as u64;
        if current_ledger < escrow.locked_until {
            panic_with_error!(e, EscrowError::FundsLocked);
        }

        // Transfer funds
        Self::transfer_token(
            e,
            &escrow.asset,
            &escrow.owner,
            &recipient,
            escrow.balance,
        );

        // Remove escrow
        escrows.remove(transaction_id.clone());
        e.storage()
            .instance()
            .set(&ESCROW_ACCOUNTS, &escrows);

        // Emit event
        e.events().publish(
            (symbol_short!("ESCROW"),),
            (symbol_short!("released"), recipient, escrow.balance),
        );
    }

    /// Refund escrowed amount back to owner
    ///
    /// # Arguments
    /// * `transaction_id` - Escrow transaction ID
    pub fn refund_escrow(e: &Env, transaction_id: String) {
        let mut escrows: Map<String, EscrowAccount> = e
            .storage()
            .instance()
            .get(&ESCROW_ACCOUNTS)
            .expect("no escrows found");

        let escrow = escrows
            .get(transaction_id.clone())
            .expect_err("escrow not found");

        escrow.owner.require_auth();

        // Transfer funds back to owner
        Self::transfer_token(
            e,
            &escrow.asset,
            &escrow.owner,
            &escrow.owner,
            escrow.balance,
        );

        // Remove escrow
        escrows.remove(transaction_id.clone());
        e.storage()
            .instance()
            .set(&ESCROW_ACCOUNTS, &escrows);

        // Emit event
        e.events().publish(
            (symbol_short!("ESCROW"),),
            (symbol_short!("refunded"), escrow.owner, escrow.balance),
        );
    }

    /// Get escrow account details
    pub fn get_escrow(e: &Env, transaction_id: String) -> EscrowAccount {
        let escrows: Map<String, EscrowAccount> = e
            .storage()
            .instance()
            .get(&ESCROW_ACCOUNTS)
            .expect("no escrows found");

        escrows
            .get(transaction_id)
            .expect_err("escrow not found")
    }

    /// Check if escrow is locked
    pub fn is_locked(e: &Env, transaction_id: String) -> bool {
        let escrows: Map<String, EscrowAccount> = e
            .storage()
            .instance()
            .get(&ESCROW_ACCOUNTS)
            .expect("no escrows found");

        if let Some(escrow) = escrows.get(transaction_id) {
            let current_ledger = e.ledger().sequence() as u64;
            current_ledger < escrow.locked_until
        } else {
            false
        }
    }

    /// Transfer tokens using the fungible token contract
    fn transfer_token(e: &Env, token: &Address, from: &Address, to: &Address, amount: i128) {
        let token_client = TokenClient::new(e, token);
        if let Err(_) = token_client.transfer(from, to, &amount) {
            panic_with_error!(e, EscrowError::AssetNotSupported);
        }
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

    pub fn transfer(&self, from: &Address, to: &Address, amount: &i128) -> Result<(), EscrowError> {
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

/// Client for escrow manager contract
pub struct EscrowManagerClient<'a> {
    pub env: &'a Env,
    pub address: &'a Address,
}

impl<'a> EscrowManagerClient<'a> {
    pub fn new(env: &'a Env, address: &'a Address) -> Self {
        Self { env, address }
    }

    pub fn create_escrow(
        &self,
        transaction_id: String,
        owner: &Address,
        asset: &Address,
        amount: i128,
        locked_until: u64,
    ) {
        let args = (
            symbol_short!("create_escrow"),
            transaction_id,
            owner,
            asset,
            amount,
            locked_until,
        );
        let _: () = self.env.invoke_contract(
            &self.address,
            &args.0,
            &(&args.1, &args.2, &args.3, &args.4, &args.5),
        );
    }

    pub fn release_escrow(&self, transaction_id: String, recipient: &Address) {
        let args = (symbol_short!("esc_rel"), transaction_id, recipient);
        let _: () = self
            .env
            .invoke_contract(&self.address, &args.0, &(&args.1, &args.2));
    }
}
