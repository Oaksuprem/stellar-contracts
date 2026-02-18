# Paywow - Integration Guide

## Getting Started

### Prerequisites
- Rust 1.70+ with wasm32-unknown-unknown target
- Stellar CLI tools for deployment
- Node.js for testing utilities (optional)

### Setup

```bash
# Clone the repository
cd /workspaces/stellar-contracts/projects/paywow

# Build all contracts
cargo build --target wasm32-unknown-unknown --release

# Run tests
cargo test --all
```

## Deployment Steps

### 1. Deploy Individual Contracts

Each contract should be deployed separately to Stellar testnet/mainnet.

```bash
# Build each contract
cd contracts/payment-processor
cargo build --target wasm32-unknown-unknown --release

# The compiled WASM is at:
# target/wasm32-unknown-unknown/release/paywow_payment_processor.wasm
```

**Deployment Parameters:**

#### PaymentProcessor
```
Constructor args:
- owner: Address (admin account)
- payment_token: Address (fungible token contract)
- platform_fee_bps: u32 (e.g., 100 for 1% fee)
```

#### EscrowManager
```
Constructor args:
- owner: Address (admin account)
```

#### DisputeResolver
```
Constructor args:
- owner: Address (admin account)
- dispute_window_ledgers: u64 (e.g., 2000 for ~10 hours on testnet)
```

#### LoyaltyProgram
```
Constructor args:
- owner: Address (admin account)
```

#### PaymentOrchestrator
```
Constructor args:
- owner: Address (admin account)
- payment_processor: Address (from step 1)
- escrow_manager: Address (from step 2)
- dispute_resolver: Address (from step 3)
- loyalty_program: Address (from step 4)
- payment_token: Address (fungible token contract)
```

### 2. Deploy Supporting Contracts

Create or use existing fungible token contracts:

```bash
# Example: Use fungible-pausable from examples
cd examples/fungible-pausable
cargo build --target wasm32-unknown-unknown --release
```

Then deploy using Stellar CLI:

```bash
# Build the token contract
cargo build --target wasm32-unknown-unknown --release

# Deploy using soroban CLI
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/fungible_pausable_example.wasm
```

## Integration Testing

### Testing Framework

Tests are located in `contracts/*/src/test.rs` for unit tests.

For integration tests across contracts, create test files that:
1. Deploy all contracts in order
2. Initialize with proper addresses
3. Execute cross-contract calls
4. Verify state changes

### Running Tests

```bash
# Run all tests
cargo test --all

# Run specific contract tests
cd contracts/payment-processor
cargo test

# Run with output
cargo test -- --nocapture
```

### Test Example

```rust
#[test]
fn test_complete_payment_flow() {
    let env = Env::default();
    
    // Create test accounts
    let owner = Address::random(&env);
    let payer = Address::random(&env);
    let payee = Address::random(&env);
    
    // Deploy all contracts
    let processor = deploy_payment_processor(&env, &owner);
    let escrow = deploy_escrow_manager(&env, &owner);
    let dispute = deploy_dispute_resolver(&env, &owner);
    let loyalty = deploy_loyalty_program(&env, &owner);
    let token = deploy_token(&env, &owner);
    
    // Deploy orchestrator
    let orchestrator = deploy_orchestrator(
        &env, &owner, &processor, &escrow, &dispute, &loyalty, &token
    );
    
    // Test simple payment
    orchestrator.process_simple_payment(
        &payer, &payee, 1000, &payee, 100
    );
    
    // Verify transaction was logged
    let tx = orchestrator.get_transaction("tx1");
    assert_eq!(tx.amount, 1000);
    assert_eq!(tx.payer, payer);
}
```

## Cross-Contract Call Examples

### Example 1: Simple Payment with Fees and Loyalty

```rust
// User initiates payment through orchestrator
let orchestrator_client = PaymentOrchestratorClient::new(&env, &orchestrator_addr);

orchestrator_client.process_simple_payment(
    &payer_address,
    &payee_address,
    1000i128,              // 1000 units
    &merchant_address,
    100u32,                // 1% merchant fee
    payment_id.clone()
);

// This internally calls:
// 1. PaymentProcessor::process_payment()
//    - Transfers 1000 to payee
//    - Transfers 10 (1% platform fee) to owner
//    - Transfers 10 (merchant fee) to merchant
// 
// 2. LoyaltyProgram::award_points()
//    - Awards 10 loyalty points (1 per 100 units)
```

### Example 2: Escrow-Protected Payment

```rust
let current_ledger = env.ledger().sequence();
let locked_until = current_ledger + 2000; // Lock for ~10 hours

orchestrator_client.process_escrow_payment(
    &payer_address,
    &payee_address,
    10000i128,
    locked_until
);

// Later: Release escrow when conditions are met
orchestrator_client.release_escrow(
    &escrow_transaction_id,
    &payee_address
);

// Or refund if needed
escrow_client.refund_escrow(&escrow_transaction_id);
```

### Example 3: Dispute Resolution

```rust
// File dispute
let dispute_client = DisputeResolverClient::new(&env, &dispute_addr);
dispute_client.file_dispute(
    &dispute_id,
    &claimant_address,
    &respondent_address,
    1000i128,
    "Product not received".to_string(),
    "Evidence hash: abc123".to_string()
);

// Wait for resolution window (automatic timeout or admin action)
if dispute_client.is_resolvable(&dispute_id) {
    dispute_client.refund_on_timeout(&dispute_id);
}
```

### Example 4: Loyalty Point Redemption

```rust
let loyalty_client = LoyaltyProgramClient::new(&env, &loyalty_addr);

// Check customer tier
let tier = loyalty_client.get_customer_tier(&customer_address);
let points = loyalty_client.get_customer_points(&customer_address);

// Redeem points (admin only)
loyalty_contract.redeem_points(&customer_address, 500);
```

## Real-World Usage Scenarios

### E-Commerce Store

1. **Browse & Checkout**
   - Customer selects items
   - Initiates payment through PaymentOrchestrator

2. **Payment Processing**
   - PaymentOrchestrator receives payment request
   - Creates payment transaction
   - If high-value: Uses EscrowManager to hold funds

3. **Fund Release**
   - Store confirms shipment
   - PaymentOrchestrator releases escrow to store owner
   - Customer receives loyalty NFT

4. **Dispute Handling**
   - If customer files dispute within window
   - DisputeResolver holds refund for 10 hours
   - Auto-refunds if not resolved by store

### Marketplace Platform

1. **Listing & Matching**
   - Sellers list items
   - Buyers browse and select

2. **Transaction Processing**
   - Payment flows through PaymentOrchestrator
   - EscrowManager holds funds
   - Platform takes fee via PaymentProcessor

3. **Reputation System**
   - Successful transactions award loyalty points
   - LoyaltyProgram tracks buyer/seller reputation
   - Points redeemable for platform credits

4. **Escrow Release**
   - Buyer confirms delivery
   - Funds released to seller
   - Both parties receive loyalty NFTs

### B2B Payment System

1. **Vendor Integration**
   - Vendors integrate via API
   - Use PaymentOrchestrator for payments

2. **Bulk Payments**
   - Company pays multiple vendors
   - Each payment logged and tracked

3. **Reconciliation**
   - Pull transaction history from PaymentOrchestrator
   - Verify payments in blockchain
   - Export for accounting

## Monitoring & Administration

### View Transaction History

```rust
// Get specific transaction
let tx = orchestrator.get_transaction(&transaction_id);
println!("Transaction: {:?}", tx);

// Get configuration
let (processor, escrow, dispute, loyalty, token) = orchestrator.get_config();
```

### Manage Payments

```rust
// Withdraw collected fees
payment_processor.withdraw_fees(&fee_amount);

// Check collected fees
let fees = payment_processor.get_collected_fees();
```

### Emergency Controls

```rust
// Pause dispute resolution
dispute_resolver.pause(&e, &owner);

// Unpause when ready
dispute_resolver.unpause(&e, &owner);
```

## Troubleshooting

### Common Issues

1. **"Contract not found"**
   - Verify contract Address is correctly set
   - Ensure contract is deployed

2. **"Unauthorized"**
   - Verify caller has required authorization
   - Check `require_auth()` is called with correct account

3. **"FundsLocked" in escrow**
   - Ledger sequence hasn't reached unlock time
   - Wait or check `is_locked()` before trying release

4. **"NotYetResolvable" in disputes**
   - Dispute window hasn't ended
   - Check `is_resolvable()` or wait for timelock

## Performance Considerations

- **Ledger Cost**: Each state change costs XLM
- **Batch Operations**: Group multiple payments to reduce overhead
- **Storage**: Consider archiving old transactions periodically
- **Event Emission**: Events are free but indexed, monitor size

## Security Checklist

- [ ] All owner addresses properly set
- [ ] Platform fee reasonable (typically 0.5-2%)
- [ ] Dispute window appropriate for use case
- [ ] Token contract is trusted/audited
- [ ] Fee recipient addresses verified
- [ ] Whitelist updated before accepting new tokens
- [ ] Emergency pause tested
- [ ] Key rotation procedure documented

## Support & References

- [Stellar Soroban Docs](https://developers.stellar.org/docs/build/smart-contracts)
- [OpenZeppelin Stellar Contracts](https://docs.openzeppelin.com/stellar-contracts/)
- [Paywow Architecture](./ARCHITECTURE.md)
- [Paywow README](./README.md)
