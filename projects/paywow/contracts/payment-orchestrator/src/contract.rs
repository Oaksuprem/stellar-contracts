//! Payment Orchestrator Contract
//!
//! Main orchestration contract that coordinates cross-contract calls to:
//! - PaymentProcessor: Process payments with fees
//! - EscrowManager: Hold funds securely
//! - DisputeResolver: Handle disputes
//! - LoyaltyProgram: Award loyalty points
//!
//! This contract demonstrates real-world usage of composable smart contracts.

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, Env, Map,
    Symbol, String,
};
use stellar_macros::only_owner;

// Storage keys
pub const ORCHESTRATOR_OWNER: Symbol = symbol_short!("OWNER");
pub const PAYMENT_PROCESSOR: Symbol = symbol_short!("PAYMENT");
pub const ESCROW_MANAGER: Symbol = symbol_short!("ESCROW");
pub const DISPUTE_RESOLVER: Symbol = symbol_short!("DISPUTE");
pub const LOYALTY_PROGRAM: Symbol = symbol_short!("LOYALTY");
pub const TRANSACTION_LOG: Symbol = symbol_short!("TXLOG");
pub const PAYMENT_TOKEN: Symbol = symbol_short!("TOKEN");

#[contract]
pub struct PaymentOrchestratorContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OrchestratorError {
    Unauthorized = 1,
    InvalidAmount = 2,
    ContractNotSet = 3,
    RefuseTransfer = 4,
    TransactionFailed = 5,
    InvalidTransactionType = 6,
}

/// Transaction status
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,      // 0
    Completed,    // 1
    EscrowHeld,   // 2
    Disputed,     // 3
    Refunded,     // 4
}

/// Transaction record
#[derive(Clone)]
pub struct Transaction {
    pub transaction_id: String,
    pub payer: Address,
    pub payee: Address,
    pub amount: i128,
    pub status: u32,
    pub transaction_type: u32, // 0: Simple, 1: Escrow, 2: Conditional
    pub created_at: u64,
}

#[contractimpl]
impl PaymentOrchestratorContract {
    /// Initialize the orchestrator with contract addresses
    pub fn __constructor(
        e: &Env,
        owner: Address,
        payment_processor: Address,
        escrow_manager: Address,
        dispute_resolver: Address,
        loyalty_program: Address,
        payment_token: Address,
    ) {
        e.storage().instance().set(&ORCHESTRATOR_OWNER, &owner);
        e.storage()
            .instance()
            .set(&PAYMENT_PROCESSOR, &payment_processor);
        e.storage()
            .instance()
            .set(&ESCROW_MANAGER, &escrow_manager);
        e.storage()
            .instance()
            .set(&DISPUTE_RESOLVER, &dispute_resolver);
        e.storage()
            .instance()
            .set(&LOYALTY_PROGRAM, &loyalty_program);
        e.storage()
            .instance()
            .set(&PAYMENT_TOKEN, &payment_token);

        let transactions: Map<String, Transaction> = Map::new(e);
        e.storage()
            .instance()
            .set(&TRANSACTION_LOG, &transactions);
    }

    /// Process a simple payment (direct transfer)
    ///
    /// # Arguments
    /// * `transaction_id` - Unique transaction identifier
    /// * `payer` - Payer address
    /// * `payee` - Payee address
    /// * `amount` - Payment amount
    /// * `merchant` - Merchant for fee split (can be same as payee)
    /// * `merchant_fee_bps` - Merchant fee in basis points
    pub fn process_simple_payment(
        e: &Env,
        transaction_id: String,
        payer: Address,
        payee: Address,
        amount: i128,
        merchant: Address,
        merchant_fee_bps: u32,
    ) {
        payer.require_auth();

        if amount <= 0 {
            panic_with_error!(e, OrchestratorError::InvalidAmount);
        }

        // Call PaymentProcessor contract
        let processor_addr = Self::get_payment_processor(e);
        Self::call_payment_processor(
            e,
            &processor_addr,
            &payer,
            &payee,
            amount,
            &merchant,
            merchant_fee_bps,
            transaction_id.clone(),
        );

        // Award loyalty points to buyer
        let loyalty_addr = Self::get_loyalty_program(e);
        let points = Self::calculate_loyalty_points(amount);
        Self::call_loyalty_program(e, &loyalty_addr, &payer, transaction_id.clone(), points);

        // Log transaction
        Self::log_transaction(e, transaction_id, payer, payee, amount, 0);

        // Emit event
        e.events().publish(
            (symbol_short!("PAYMENT"),),
            (
                symbol_short!("simple"),
                payer,
                payee,
                amount,
            ),
        );
    }

    /// Process payment with escrow (secure transaction)
    ///
    /// # Arguments
    /// * `transaction_id` - Unique transaction identifier
    /// * `payer` - Payer address
    /// * `payee` - Payee address
    /// * `amount` - Payment amount
    /// * `locked_until` - Ledger sequence until funds are locked
    pub fn process_escrow_payment(
        e: &Env,
        transaction_id: String,
        payer: Address,
        payee: Address,
        amount: i128,
        locked_until: u64,
    ) {
        payer.require_auth();

        if amount <= 0 {
            panic_with_error!(e, OrchestratorError::InvalidAmount);
        }

        // Create escrow
        let escrow_addr = Self::get_escrow_manager(e);
        let token_addr = Self::get_payment_token(e);
        Self::call_create_escrow(
            e,
            &escrow_addr,
            transaction_id.clone(),
            &payer,
            &token_addr,
            amount,
            locked_until,
        );

        // Award loyalty points
        let loyalty_addr = Self::get_loyalty_program(e);
        let points = Self::calculate_loyalty_points(amount);
        Self::call_loyalty_program(e, &loyalty_addr, &payer, transaction_id.clone(), points);

        // Log transaction
        Self::log_transaction(e, transaction_id, payer, payee, amount, 1);

        // Emit event
        e.events().publish(
            (symbol_short!("PAYMENT"),),
            (
                symbol_short!("escrow"),
                payer,
                payee,
                amount,
                locked_until,
            ),
        );
    }

    /// Release escrowed funds to payee
    pub fn release_escrow(
        e: &Env,
        transaction_id: String,
        payee: Address,
    ) {
        let escrow_addr = Self::get_escrow_manager(e);
        Self::call_release_escrow(e, &escrow_addr, transaction_id.clone(), &payee);

        Self::update_transaction_status(e, transaction_id, 1); // Completed

        e.events().publish(
            (symbol_short!("PAYMENT"),),
            (
                symbol_short!("esc_rel"),
                payee,
            ),
        );
    }

    /// File a dispute for a transaction
    pub fn file_dispute(
        e: &Env,
        transaction_id: String,
        claimant: Address,
        respondent: Address,
        reason: String,
        evidence: String,
    ) {
        claimant.require_auth();

        // Get transaction details
        let transaction = Self::get_transaction(e, transaction_id.clone());

        // File dispute
        let dispute_resolver_addr = Self::get_dispute_resolver(e);
        Self::call_file_dispute(
            e,
            &dispute_resolver_addr,
            transaction_id.clone(),
            &claimant,
            &respondent,
            transaction.amount,
            reason,
            evidence,
        );

        Self::update_transaction_status(e, transaction_id, 3); // Disputed

        e.events().publish(
            (symbol_short!("PAYMENT"),),
            (
                symbol_short!("disp_file"),
                claimant,
                respondent,
            ),
        );
    }

    /// Get transaction details
    pub fn get_transaction(e: &Env, transaction_id: String) -> Transaction {
        let transactions: Map<String, Transaction> = e
            .storage()
            .instance()
            .get(&TRANSACTION_LOG)
            .expect("no transactions found");

        transactions
            .get(transaction_id)
            .expect_err("transaction not found")
    }

    /// Get orchestrator configuration
    pub fn get_config(e: &Env) -> (Address, Address, Address, Address, Address) {
        (
            Self::get_payment_processor(e),
            Self::get_escrow_manager(e),
            Self::get_dispute_resolver(e),
            Self::get_loyalty_program(e),
            Self::get_payment_token(e),
        )
    }

    // ============ Internal Helper Functions ============

    fn get_payment_processor(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&PAYMENT_PROCESSOR)
            .expect("payment processor not set")
    }

    fn get_escrow_manager(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&ESCROW_MANAGER)
            .expect("escrow manager not set")
    }

    fn get_dispute_resolver(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DISPUTE_RESOLVER)
            .expect("dispute resolver not set")
    }

    fn get_loyalty_program(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&LOYALTY_PROGRAM)
            .expect("loyalty program not set")
    }

    fn get_payment_token(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&PAYMENT_TOKEN)
            .expect("payment token not set")
    }

    /// Cross-contract call to PaymentProcessor
    fn call_payment_processor(
        e: &Env,
        processor: &Address,
        from: &Address,
        to: &Address,
        amount: i128,
        merchant: &Address,
        merchant_fee_bps: u32,
        payment_id: String,
    ) {
        let args = (
            symbol_short!("pay_proc"),
            from,
            to,
            amount,
            merchant,
            merchant_fee_bps,
            payment_id,
        );
        let _: () = e.invoke_contract(
            processor,
            &args.0,
            &(&args.1, &args.2, &args.3, &args.4, &args.5, &args.6),
        );
    }

    /// Cross-contract call to EscrowManager - create escrow
    fn call_create_escrow(
        e: &Env,
        escrow: &Address,
        transaction_id: String,
        owner: &Address,
        asset: &Address,
        amount: i128,
        locked_until: u64,
    ) {
        let args = (
            symbol_short!("esc_crt"),
            transaction_id,
            owner,
            asset,
            amount,
            locked_until,
        );
        let _: () = e.invoke_contract(
            escrow,
            &args.0,
            &(&args.1, &args.2, &args.3, &args.4, &args.5),
        );
    }

    /// Cross-contract call to EscrowManager - release escrow
    fn call_release_escrow(
        e: &Env,
        escrow: &Address,
        transaction_id: String,
        recipient: &Address,
    ) {
        let args = (symbol_short!("esc_rel"), transaction_id, recipient);
        let _: () = e.invoke_contract(
            escrow,
            &args.0,
            &(&args.1, &args.2),
        );
    }

    /// Cross-contract call to DisputeResolver
    fn call_file_dispute(
        e: &Env,
        dispute_resolver: &Address,
        dispute_id: String,
        claimant: &Address,
        respondent: &Address,
        amount: i128,
        reason: String,
        evidence: String,
    ) {
        let args = (
            symbol_short!("disp_file"),
            dispute_id,
            claimant,
            respondent,
            amount,
            reason,
            evidence,
        );
        let _: () = e.invoke_contract(
            dispute_resolver,
            &args.0,
            &(&args.1, &args.2, &args.3, &args.4, &args.5, &args.6),
        );
    }

    /// Cross-contract call to LoyaltyProgram
    fn call_loyalty_program(
        e: &Env,
        loyalty: &Address,
        customer: &Address,
        transaction_id: String,
        points: u32,
    ) {
        let args = (
            symbol_short!("awd_pts"),
            customer,
            transaction_id,
            points,
        );
        let _: u32 = e.invoke_contract(
            loyalty,
            &args.0,
            &(&args.1, &args.2, &args.3),
        );
    }

    /// Calculate loyalty points (1 point per unit amount)
    fn calculate_loyalty_points(amount: i128) -> u32 {
        (amount / 100) as u32 // 1 point per 100 units
    }

    /// Log a transaction
    fn log_transaction(
        e: &Env,
        transaction_id: String,
        payer: Address,
        payee: Address,
        amount: i128,
        transaction_type: u32,
    ) {
        let mut transactions: Map<String, Transaction> = e
            .storage()
            .instance()
            .get(&TRANSACTION_LOG)
            .unwrap_or(Map::new(e));

        let transaction = Transaction {
            transaction_id: transaction_id.clone(),
            payer,
            payee,
            amount,
            status: 0, // Pending
            transaction_type,
            created_at: e.ledger().sequence() as u64,
        };

        transactions.set(transaction_id, transaction);
        e.storage()
            .instance()
            .set(&TRANSACTION_LOG, &transactions);
    }

    /// Update transaction status
    fn update_transaction_status(
        e: &Env,
        transaction_id: String,
        status: u32,
    ) {
        let mut transactions: Map<String, Transaction> = e
            .storage()
            .instance()
            .get(&TRANSACTION_LOG)
            .expect("no transactions found");

        let mut transaction = transactions
            .get(transaction_id.clone())
            .expect_err("transaction not found");

        transaction.status = status;
        transactions.set(transaction_id, transaction);
        e.storage()
            .instance()
            .set(&TRANSACTION_LOG, &transactions);
    }
}
