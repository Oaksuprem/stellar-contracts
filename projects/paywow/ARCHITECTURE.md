# Paywow Smart Contracts Architecture

## System Overview

Paywow is a decentralized payment platform built on Stellar using Soroban smart contracts. It demonstrates real-world integration of multiple smart contracts working together through cross-contract calls.

```
┌─────────────────────────────────────────────────────────────┐
│                  PaymentOrchestrator                        │
│         (Main contract coordinating interactions)           │
└──────────────────┬──────────────────────────────────────────┘
                   │
        ┌──────────┼──────────┬──────────────┐
        │          │          │              │
        ▼          ▼          ▼              ▼
    ┌────────┐ ┌────────┐ ┌────────┐ ┌──────────┐
    │Payment │ │Escrow  │ │Dispute │ │ Loyalty  │
    │Process │ │Manager │ │Resolver│ │ Program  │
    └────────┘ └────────┘ └────────┘ └──────────┘
        │
        ▼
    OpenZeppelin Packages
    - Fungible Tokens
    - Vault (Escrow)
    - Access Control
    - Governance (Timelock)
```

## Contract Details

### 1. PaymentProcessor
**Purpose:** Core payment processing with fee abstraction

**Key Features:**
- Process direct payments between accounts
- Dynamic fee calculation (platform + merchant fees)
- Fee collection and withdrawal
- Token whitelist management

**Cross-Contract Calls:**
- Called by: `PaymentOrchestrator`, External callers
- Calls: Fungible token contracts (transfer)

**Storage:**
- `OWNER`: Admin address
- `PAYMENT_TOKEN`: Token used for payments
- `PLATFORM_FEE`: Platform fee in basis points (100 = 1%)
- `COLLECTED_FEES`: Total collected platform fees
- `WHITELIST`: Supported tokens map

**Events:**
- `PAYMENT`: PaymentProcessed event with sender, recipient, amounts

### 2. EscrowManager
**Purpose:** Secure fund holding for transactions

**Key Features:**
- Create escrow accounts with time locks
- Release funds when conditions are met
- Refund escrowed amounts
- Support for multiple assets

**Cross-Contract Calls:**
- Called by: `PaymentOrchestrator`, External callers
- Calls: Fungible token contracts (transfer)

**Storage:**
- `ESCROW_OWNER`: Admin address
- `ESCROW_ACCOUNTS`: Map of transaction ID → EscrowAccount
- `ESCROW_LOCKED_UNTIL`: Lock time for each escrow

**Events:**
- `ESCROW`: Created, released, refunded events

### 3. DisputeResolver
**Purpose:** Handle disputes with timelock-based resolution

**Key Features:**
- File disputes with evidence
- Timelock-based dispute windows
- Admin-based resolution
- Automatic refunds on timeout

**Cross-Contract Calls:**
- Called by: `PaymentOrchestrator`, External callers
- Calls: None directly (can call refund contracts)

**Storage:**
- `DISPUTE_OWNER`: Admin address
- `DISPUTES`: Map of dispute ID → Dispute
- `DISPUTE_WINDOW`: Ledger sequence duration for disputes

**Events:**
- `DISPUTE`: Filed, resolved, refunded events

### 4. LoyaltyProgram
**Purpose:** NFT-based loyalty rewards

**Key Features:**
- Award loyalty points for transactions
- Tier-based loyalty system (Bronze → Platinum)
- Issue NFT loyalty cards
- Redeem points for rewards

**Cross-Contract Calls:**
- Called by: `PaymentOrchestrator`, External callers
- Calls: None directly (uses internal NFT storage)

**Storage:**
- `LOYALTY_OWNER`: Admin address
- `CUSTOMER_POINTS`: Map of customer → points
- `ISSUED_REWARDS`: Map of token ID → LoyaltyReward
- `TOTAL_REWARDS`: Total NFTs issued

**Events:**
- `LOYALTY`: Points awarded, NFT issued, points redeemed

### 5. PaymentOrchestrator
**Purpose:** Main contract orchestrating all interactions

**Features:**
- Simple payment processing
- Escrow-based secure transactions
- Dispute filing and management
- Automatic loyalty point awards

**Cross-Contract Calls Made:**
- `PaymentProcessor::process_payment()` - for fee-based transfers
- `EscrowManager::create_escrow()` - for escrow creation
- `EscrowManager::release_escrow()` - for fund release
- `DisputeResolver::file_dispute()` - for dispute filing
- `LoyaltyProgram::award_points()` - for loyalty points

**Storage:**
- `ORCHESTRATOR_OWNER`: Admin address
- `PAYMENT_PROCESSOR`: PaymentProcessor address
- `ESCROW_MANAGER`: EscrowManager address
- `DISPUTE_RESOLVER`: DisputeResolver address
- `LOYALTY_PROGRAM`: LoyaltyProgram address
- `PAYMENT_TOKEN`: Default payment token
- `TRANSACTION_LOG`: Map of transaction ID → Transaction

**Events:**
- `PAYMENT`: Simple, escrow, dispute filed, escrow released events

## Cross-Contract Call Flows

### Flow 1: Simple Payment
```
PaymentOrchestrator.process_simple_payment()
├── PaymentProcessor.process_payment()
│   └── TokenContract.transfer()
└── LoyaltyProgram.award_points()
```

### Flow 2: Escrow Payment
```
PaymentOrchestrator.process_escrow_payment()
├── EscrowManager.create_escrow()
│   └── TokenContract.transfer() [to escrow]
└── LoyaltyProgram.award_points()
```

### Flow 3: Escrow Release
```
PaymentOrchestrator.release_escrow()
├── EscrowManager.release_escrow()
│   └── TokenContract.transfer() [to recipient]
└── Update transaction status
```

### Flow 4: Dispute Filing
```
PaymentOrchestrator.file_dispute()
├── DisputeResolver.file_dispute()
└── Update transaction status
```

## Data Structures

### PaymentProcessed Event
```rust
pub struct PaymentProcessed {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
    pub platform_fee: i128,
    pub merchant_fee: i128,
    pub payment_id: String,
}
```

### EscrowAccount
```rust
pub struct EscrowAccount {
    pub owner: Address,
    pub balance: i128,
    pub asset: Address,
    pub locked_until: u64,
    pub transaction_id: String,
}
```

### Dispute
```rust
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
```

### LoyaltyReward
```rust
pub struct LoyaltyReward {
    pub token_id: u32,
    pub owner: Address,
    pub points_earned: u32,
    pub tier: u32,
    pub transaction_id: String,
    pub issued_at: u64,
}
```

### Transaction
```rust
pub struct Transaction {
    pub transaction_id: String,
    pub payer: Address,
    pub payee: Address,
    pub amount: i128,
    pub status: u32,
    pub transaction_type: u32, // 0: Simple, 1: Escrow, 2: Conditional
    pub created_at: u64,
}
```

## Error Handling

Each contract defines specific error types:

**PaymentError:**
- `Unauthorized`: Caller not authorized
- `InvalidAmount`: Amount <= 0
- `TokenNotSupported`: Token not in whitelist
- `FeeExceedsPayment`: Fees >= payment amount
- `InsufficientBalance`: Account balance too low
- `TransferFailed`: Token transfer failed

**EscrowError:**
- Similar structure with escrow-specific errors

**DisputeError:**
- `NotYetResolvable`: Dispute window not ended
- `AlreadyResolved`: Dispute already has outcome

**LoyaltyError:**
- `InvalidPoints`: Points value invalid
- `NoRewardEarned`: Insufficient points for reward

## Security Considerations

1. **Authentication:** All sensitive operations require `require_auth()` checks
2. **Authorization:** Owner-only operations use `#[only_owner]` macro
3. **Fee Validation:** Platform fee capped at 100% (10000 bps)
4. **Pausable:** DisputeResolver can be paused for emergency stops
5. **Timelock:** Disputes have automatic resolution timeout
6. **Cross-contract Safety:** All contracts validate addresses before calls

## Deployment Order

1. Deploy token contract (if not using existing)
2. Deploy `PaymentProcessor`
3. Deploy `EscrowManager`
4. Deploy `DisputeResolver`
5. Deploy `LoyaltyProgram`
6. Deploy `PaymentOrchestrator` with addresses of above contracts
7. Initialize contract relationships

## Testing Strategy

- Unit tests for each contract
- Integration tests for cross-contract calls
- Mock contract clients for testing interactions
- Event emission verification
- Error condition testing

## Future Enhancements

- Multi-currency support with exchange rates
- Atomic swaps between escrow accounts
- Advanced access control with roles
- Governance for fee updates
- Merchant tiers and volume discounts
- KYC/AML integration hooks
