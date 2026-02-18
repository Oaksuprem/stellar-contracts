# Paywow - Decentralized Payment Platform

Paywow is a comprehensive decentralized payment platform built on the Stellar network using Soroban smart contracts. It leverages OpenZeppelin's Stellar contracts to provide secure, scalable, and composable payment solutions.

## Overview

Paywow demonstrates real-world usage of multiple smart contracts working together through cross-contract calls to enable:

- **Payment Processing**: Secure payment transfers with fee abstraction
- **Escrow Management**: Secure fund management for trades and transactions
- **Dispute Resolution**: Timelock-based dispute handling mechanisms
- **Loyalty Programs**: NFT-based loyalty rewards and incentives
- **Smart Account Management**: Advanced smart accounts for merchants and users

## Architecture

```
PaymentOrchestrator (Main Contract)
├── PaymentProcessor (Fee-based transactions)
├── EscrowManager (Fund holding for transactions)
├── DisputeResolver (Timelock-based dispute handling)
├── LoyaltyProgram (NFT rewards)
└── Smart Accounts (Merchant accounts)
```

## Contracts

### 1. PaymentProcessor
Handles core payment functionality with fee abstraction. Built using:
- `stellar_tokens::fungible`: Token transfer functionality
- `stellar_fee_abstraction`: Cover fees with fungible tokens
- `stellar_access`: Ownable patterns for admin controls

**Key Features:**
- Process payments between accounts
- Dynamic fee calculation and collection
- Fee distribution to merchants and platform
- Token whitelist management

### 2. EscrowManager
Manages escrow accounts for secure transactions. Built using:
- `stellar_tokens::vault`: Tokenized vault for secure fund holding
- `stellar_tokens::fungible`: Underlying token management
- `stellar_access`: Role-based access control

**Key Features:**
- Create and manage escrow accounts
- Lock funds for transactions
- Release funds when conditions are met
- Support for multiple assets

### 3. DisputeResolver
Handles dispute resolution with timelocks. Built using:
- `stellar_governance::timelock`: Delayed execution for disputes
- `stellar_access`: Ownable patterns for dispute adjudication
- `stellar_contract_utils::pausable`: Emergency pause mechanism

**Key Features:**
- File disputes with evidence
- Timelock-based dispute windows
- Refund processing for unresolved disputes
- Escalation to arbitration

### 4. LoyaltyProgram
NFT-based loyalty rewards system. Built using:
- `stellar_tokens::non_fungible`: NFT standards
- `stellar_tokens::mint`: NFT minting for rewards
- `stellar_access`: Role-based permissions

**Key Features:**
- Issue loyalty NFTs for transactions
- Track merchant reputation through NFTs
- Tier-based rewards system
- NFT metadata for transaction details

### 5. PaymentOrchestrator
Main contract that orchestrates all other contracts through cross-contract calls.

**Key Features:**
- Coordinate payment flows
- Manage transaction states
- Ensure atomicity across contracts
- Handle complex payment scenarios (escrow + dispute resolution)

## Cross-Contract Calls

The contracts use cross-contract calls via Soroban's `ContractClient` to interact:

```rust
// Example: Payment with escrow
1. PaymentOrchestrator receives payment request
2. Calls PaymentProcessor to calculate fees
3. Calls EscrowManager to hold funds
4. Calls LoyaltyProgram to issue rewards
5. Returns transaction reference

// Example: Dispute Resolution
1. DisputeResolver receives dispute
2. Sets timelock window for resolution
3. On timeout, calls PaymentProcessor to refund
4. Updates LoyaltyProgram stake if applicable
```

## Building

```bash
# Build all contracts
cargo build --target wasm32-unknown-unknown --release

# Build specific contract
cd contracts/payment-processor
cargo build --target wasm32-unknown-unknown --release
```

## Testing

```bash
# Run all tests
cargo test --all

# Run specific contract tests
cd contracts/payment-processor
cargo test
```

## Integration Testing

Cross-contract integration tests verify that contracts work together correctly:

```bash
cargo test --test integration_tests
```

## Deployment

Each contract is deployed independently to Stellar testnet:

1. Deploy PaymentProcessor
2. Deploy EscrowManager
3. Deploy DisputeResolver
4. Deploy LoyaltyProgram
5. Deploy PaymentOrchestrator with addresses of above contracts
6. Initialize contract relationships

## Use Cases

### E-Commerce
- Buyer sends payment through PaymentOrchestrator
- Funds held in EscrowManager until shipment confirmed
- Seller receives payment on confirmation
- Buyer receives loyalty NFT

### Marketplace
- Marketplace takes percentage fee via PaymentProcessor
- Multiple currencies handled via token abstractions
- Disputes resolved through DisputeResolver
- Top traders rewarded with loyalty NFTs

### SaaS Subscriptions
- Recurring payments via PaymentProcessor
- Failed payment handling via DisputeResolver
- Customer status via LoyaltyProgram NFTs
- Merchant management via smart accounts

## Security Considerations

- All contracts follow OpenZeppelin security patterns
- Pausable functionality for emergency stops
- Ownable patterns for admin controls
- Timelock delays for critical operations
- Cross-contract call validation

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md)

## License

MIT - See [LICENSE](../../LICENSE)
