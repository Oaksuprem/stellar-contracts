//! Dispute Resolver Contract
//!
//! Handles disputes with timelock-based resolution mechanisms.
//! Integrates with OpenZeppelin's governance (timelock) and access control.

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, Env, Map,
    Symbol, String, IntoVal, TryFromVal,
};
use stellar_contract_utils::pausable::Pausable;

// Storage keys
pub const DISPUTE_OWNER: Symbol = symbol_short!("OWNER");
pub const DISPUTES: Symbol = symbol_short!("DISPUTES");
pub const DISPUTE_WINDOW: Symbol = symbol_short!("DWINDOW");

#[contract]
pub struct DisputeResolverContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DisputeError {
    Unauthorized = 1,
    DisputeNotFound = 2,
    InvalidAmount = 3,
    NotYetResolvable = 4,
    AlreadyResolved = 5,
    InvalidEvidence = 6,
}

/// Dispute status
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DisputeStatus {
    Filed,
    UnderReview,
    Resolved,
    Refunded,
}

/// Dispute record
#[derive(Clone, IntoVal, TryFromVal)]
#[soroban_sdk::contracttype]
pub struct Dispute {
    pub dispute_id: String,
    pub claimant: Address,
    pub respondent: Address,
    pub amount: i128,
    pub reason: String,
    pub evidence: String,
    pub filed_at: u64,
    pub resolution_deadline: u64,
    pub status: u32, // 0: Filed, 1: UnderReview, 2: Resolved, 3: Refunded
}

#[contractimpl]
impl DisputeResolverContract {
    /// Initialize the dispute resolver
    ///
    /// # Arguments
    /// * `owner` - Admin address
    /// * `dispute_window_ledgers` - Number of ledgers for dispute resolution
    pub fn __constructor(e: &Env, owner: Address, dispute_window_ledgers: u64) {
        e.storage().instance().set(&DISPUTE_OWNER, &owner);
        e.storage()
            .instance()
            .set(&DISPUTE_WINDOW, &dispute_window_ledgers);
        let disputes: Map<String, Dispute> = Map::new(e);
        e.storage().instance().set(&DISPUTES, &disputes);
    }

    /// File a new dispute
    ///
    /// # Arguments
    /// * `dispute_id` - Unique dispute identifier
    /// * `claimant` - Address filing the dispute
    /// * `respondent` - Address being disputed
    /// * `amount` - Amount in dispute
    /// * `reason` - Reason for dispute
    /// * `evidence` - Supporting evidence/documentation
    pub fn file_dispute(
        e: &Env,
        dispute_id: String,
        claimant: Address,
        respondent: Address,
        amount: i128,
        reason: String,
        evidence: String,
    ) {
        claimant.require_auth();

        if amount <= 0 {
            panic_with_error!(e, DisputeError::InvalidAmount);
        }

        let dispute_window: u64 = e
            .storage()
            .instance()
            .get(&DISPUTE_WINDOW)
            .expect("dispute window not set");

        let current_ledger = e.ledger().sequence() as u64;
        let resolution_deadline = current_ledger + dispute_window;

        let mut disputes: Map<String, Dispute> = e
            .storage()
            .instance()
            .get(&DISPUTES)
            .unwrap_or(Map::new(e));

        let dispute = Dispute {
            dispute_id: dispute_id.clone(),
            claimant: claimant.clone(),
            respondent,
            amount,
            reason,
            evidence,
            filed_at: current_ledger,
            resolution_deadline,
            status: 0, // Filed
        };

        disputes.set(dispute_id.clone(), dispute);
        e.storage().instance().set(&DISPUTES, &disputes);

        // Emit event
        e.events().publish(
            (symbol_short!("DISPUTE"),),
            (
                symbol_short!("filed"),
                claimant,
                amount,
                resolution_deadline,
            ),
        );
    }

    /// Resolve a dispute (admin function)
    ///
    /// # Arguments
    /// * `dispute_id` - ID of dispute to resolve
    /// * `ruling` - Address to receive the refund (claimant = 1, respondent = 0)
    #[only_owner]
    pub fn resolve_dispute(e: &Env, dispute_id: String, refund_claimant: bool) {
        let mut disputes: Map<String, Dispute> = e
            .storage()
            .instance()
            .get(&DISPUTES)
            .expect("no disputes found");

        let mut dispute = disputes
            .get(dispute_id.clone())
            .expect_err("dispute not found");

        if dispute.status != 0 && dispute.status != 1 {
            panic_with_error!(e, DisputeError::AlreadyResolved);
        }

        dispute.status = 2; // Resolved
        disputes.set(dispute_id.clone(), dispute.clone());
        e.storage().instance().set(&DISPUTES, &disputes);

        // Determine refund recipient
        let recipient = if refund_claimant {
            dispute.claimant.clone()
        } else {
            dispute.respondent.clone()
        };

        // Emit event
        e.events().publish(
            (symbol_short!("DISPUTE"),),
            (
                symbol_short!("resolved"),
                dispute_id,
                recipient,
                dispute.amount,
            ),
        );
    }

    /// Auto-refund if dispute window expires without resolution
    ///
    /// # Arguments
    /// * `dispute_id` - ID of dispute to auto-refund
    pub fn refund_on_timeout(e: &Env, dispute_id: String) {
        let mut disputes: Map<String, Dispute> = e
            .storage()
            .instance()
            .get(&DISPUTES)
            .expect("no disputes found");

        let mut dispute = disputes
            .get(dispute_id.clone())
            .expect_err("dispute not found");

        let current_ledger = e.ledger().sequence() as u64;

        if current_ledger < dispute.resolution_deadline {
            panic_with_error!(e, DisputeError::NotYetResolvable);
        }

        if dispute.status == 3 {
            panic_with_error!(e, DisputeError::AlreadyResolved);
        }

        dispute.status = 3; // Refunded
        disputes.set(dispute_id.clone(), dispute.clone());
        e.storage().instance().set(&DISPUTES, &disputes);

        // Emit event - refund goes to claimant on timeout
        e.events().publish(
            (symbol_short!("DISPUTE"),),
            (
                symbol_short!("refunded"),
                dispute_id,
                dispute.claimant,
                dispute.amount,
            ),
        );
    }

    /// Get dispute details
    pub fn get_dispute(e: &Env, dispute_id: String) -> Dispute {
        let disputes: Map<String, Dispute> = e
            .storage()
            .instance()
            .get(&DISPUTES)
            .expect("no disputes found");

        disputes
            .get(dispute_id)
            .expect_err("dispute not found")
    }

    /// Check if dispute is resolvable
    pub fn is_resolvable(e: &Env, dispute_id: String) -> bool {
        let disputes: Map<String, Dispute> = e
            .storage()
            .instance()
            .get(&DISPUTES)
            .expect("no disputes found");

        if let Some(dispute) = disputes.get(dispute_id) {
            let current_ledger = e.ledger().sequence() as u64;
            current_ledger >= dispute.resolution_deadline
        } else {
            false
        }
    }
}

#[contractimpl]
impl Pausable for DisputeResolverContract {
    fn paused(e: &Env) -> bool {
        pausable::paused(e)
    }

    fn pause(e: &Env, caller: Address) {
        caller.require_auth();
        let owner: Address = e
            .storage()
            .instance()
            .get(&DISPUTE_OWNER)
            .expect("owner should be set");
        if owner != caller {
            panic_with_error!(e, DisputeError::Unauthorized);
        }
        pausable::pause(e);
    }

    fn unpause(e: &Env, caller: Address) {
        caller.require_auth();
        let owner: Address = e
            .storage()
            .instance()
            .get(&DISPUTE_OWNER)
            .expect("owner should be set");
        if owner != caller {
            panic_with_error!(e, DisputeError::Unauthorized);
        }
        pausable::unpause(e);
    }
}

/// Client for dispute resolver contract
pub struct DisputeResolverClient<'a> {
    pub env: &'a Env,
    pub address: &'a Address,
}

impl<'a> DisputeResolverClient<'a> {
    pub fn new(env: &'a Env, address: &'a Address) -> Self {
        Self { env, address }
    }

    pub fn file_dispute(
        &self,
        dispute_id: String,
        claimant: &Address,
        respondent: &Address,
        amount: i128,
        reason: String,
        evidence: String,
    ) {
        let args = (
            symbol_short!("file_dispute"),
            dispute_id,
            claimant,
            respondent,
            amount,
            reason,
            evidence,
        );
        let _: () = self.env.invoke_contract(
            &self.address,
            &args.0,
            &(&args.1, &args.2, &args.3, &args.4, &args.5, &args.6),
        );
    }

    pub fn refund_on_timeout(&self, dispute_id: String) {
        let args = (symbol_short!("rfnd_tout"), dispute_id);
        let _: () = self
            .env
            .invoke_contract(&self.address, &args.0, &(&args.1));
    }
}
