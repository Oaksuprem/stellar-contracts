# Paywow Use Case Examples

This directory contains real-world examples of how Paywow contracts work together.

## Quick Reference

| Use Case | Contracts Used | Flow |
|----------|----------------|------|
| Simple Transfer | PaymentProcessor, LoyaltyProgram | Pay → Award Points |
| Buyer Protection | EscrowManager, DisputeResolver, PaymentProcessor | Escrow → Confirm → Release or Dispute |
| Marketplace | All Contracts | Browse → Escrow → Confirm → Release → Points |
| Subscription | PaymentProcessor, LoyaltyProgram | Recurring → Points → Tier Updates |
| Refund Flow | EscrowManager, DisputeResolver | Create → Dispute → Timeout → Refund |

## Examples

### 1. E-Commerce Simple Purchase

**Scenario:** Customer buys an ebook for 10 USDC

**Flow:**
```
Customer: 10 USDC
    ↓
PaymentOrchestrator.process_simple_payment()
    ├→ PaymentProcessor.process_payment()
    │   ├→ Customer sends 10 USDC to seller
    │   ├→ Platform fee: 0.1 USDC to owner (1%)
    │   └→ Seller receives 9.9 USDC
    └→ LoyaltyProgram.award_points()
        └→ Customer gets 1 loyalty point (1 per 100 units)

Result: Customer has ebook + 1 loyalty point
```

**Code:**
```rust
orchestrator.process_simple_payment(
    &customer,           // payer
    &seller,             // payee
    1000i128,            // 10 USDC (in units)
    &seller,             // merchant
    0u32,                // no additional merchant fee
    "ebook_purchase_001"
);
```

### 2. Marketplace with Buyer Protection

**Scenario:** Buyer purchases used phone from seller for 200 USDC with escrow

**Flow:**
```
Buyer: 200 USDC
    ↓
PaymentOrchestrator.process_escrow_payment()
    ├→ EscrowManager.create_escrow()
    │   ├→ Buyer transfers 200 USDC to escrow
    │   ├→ Locked until ledger sequence + 2000 (~10 hours)
    │   └→ Transaction ID: "phone_sale_001"
    ├→ PaymentProcessor.process_payment() [from escrow]
    │   ├→ Platform fee: 2 USDC (1%) to platform
    │   └→ Seller fee: 2 USDC (1%) to seller's account
    └→ LoyaltyProgram.award_points()
        └→ Buyer gets 2 loyalty points (1 per 100 units)

Shipping Phase:
Seller ships item
    ↓
PackageTracker confirms delivery
    ↓
Buyer confirms receipt
    ↓
PaymentOrchestrator.release_escrow()
    ├→ EscrowManager.release_escrow()
    │   └→ 196 USDC transferred to seller
    └→ LoyaltyProgram: Seller gets reputation points

Result: Buyer has item, both parties have loyalty points
```

**Code:**
```rust
let current_ledger = env.ledger().sequence() as u64;
let locked_until = current_ledger + 2000; // ~10 hours

orchestrator.process_escrow_payment(
    &"phone_sale_001".into(),
    &buyer,
    &seller,
    20000i128,           // 200 USDC in units
    locked_until
);

// ... Later after confirmation ...

orchestrator.release_escrow(
    &"phone_sale_001".into(),
    &seller
);
```

### 3. Dispute Resolution

**Scenario:** Buyer disputes transaction because item arrived damaged

**Flow:**
```
Transaction in progress
    ↓
Buyer receives damaged item
    ↓
PaymentOrchestrator.file_dispute()
    ├→ DisputeResolver.file_dispute()
    │   ├→ Files dispute with evidence/photos
    │   ├→ Sets resolution deadline to current_ledger + 2000
    │   └→ Initiates adjudication process
    └→ Update transaction status to "Disputed"

Admin Review Phase (10 hours):
Option A: Admin resolves in favor of buyer
    └→ Refund issued to buyer

Option B: Admin resolves in favor of seller
    └→ Transaction completed, seller keeps funds

Option C: No action within 10 hours
    └→ Automatic timeout refund to buyer

Result: Funds returned or retained based on resolution
```

**Code:**
```rust
orchestrator.file_dispute(
    &"phone_sale_001".into(),
    &buyer,
    &seller,
    "Item arrived damaged, not as described".into(),
    "Evidence: photo_damage_001.hash, photo_damage_002.hash"
);

// Admin later resolves
dispute_resolver.resolve_dispute(&"phone_sale_001".into(), true); // refund buyer

// Or automatic timeout:
dispute_resolver.refund_on_timeout(&"phone_sale_001".into());
```

### 4. Loyalty Tier Progression

**Scenario:** Tracking customer loyalty over multiple purchases

**Flow:**
```
Purchase 1: 100 USDC
    └→ 1 point → Bronze tier (0-9 points)

Purchase 2: 500 USDC
    └→ 5 points → Total: 6 points (Bronze)

Purchase 3: 2000 USDC
    └→ 20 points → Total: 26 points → Silver tier (10-49 points)

Purchase 4: 3000 USDC
    └→ 30 points → Total: 56 points → Gold tier (50-99 points)

Purchase 5: 5000 USDC
    └→ 50 points → Total: 106 points → Platinum tier (100+ points)
    
Platinum Status: Customer receives premium benefits
```

**Code:**
```rust
loyalty.award_points(&customer, "purchase_001", 1);   // Bronze
loyalty.award_points(&customer, "purchase_002", 5);   // Still Bronze
loyalty.award_points(&customer, "purchase_003", 20);  // Silver
loyalty.award_points(&customer, "purchase_004", 30);  // Gold
loyalty.award_points(&customer, "purchase_005", 50);  // Platinum

let tier = loyalty.get_customer_tier(&customer); // Returns 3 (Platinum)
let points = loyalty.get_customer_points(&customer); // Returns 106
```

### 5. Subscription Payment

**Scenario:** Monthly subscription payment with automatic fee handling

**Flow:**
```
Month 1:
Customer: 50 USDC/month
    ↓
PaymentOrchestrator.process_simple_payment()
    ├→ PaymentProcessor.process_payment()
    │   ├→ Fee: 0.5 USDC (1%) to platform
    │   └→ Provider: 49.5 USDC
    └→ LoyaltyProgram.award_points()
        └→ Award 5 loyalty points (1 per 100 units × 0.5)

Month 2: (Same process)
Points accumulate: 10 total

Month 12: (Same process)
Points accumulated: 120 (at Platinum tier)

Customer can redeem:
120 points → $12 credit on next payment
```

**Code:**
```rust
// Monthly automated payment
orchestrator.process_simple_payment(
    &customer,
    &provider,
    5000i128,            // 50 USDC in units
    &provider,
    0u32,
    "subscription_2024_01" // Month identifier
);

// After 12 months
let points = loyalty.get_customer_points(&customer); // 120+

// Redeem for credit
loyalty.redeem_points(&customer, 120);
// Creates $12 credit on next billing
```

### 6. Multi-Recipient Payment

**Scenario:** Selling item through marketplace with multiple stakeholders

**Flow:**
```
Buyer sends: 100 USDC
    ↓
Marketplace takes 10% (10 USDC fee)
    ├→ PaymentProcessor with merchant_fee_bps=1000
    │   └→ 10 USDC to marketplace
Seller gets: 90 USDC
    └→ Seller also pays referrer
        ├→ Process second payment
        │   └→ Seller sends 10 USDC to referrer
        └→ Seller keeps: 80 USDC

Loyalty Awards:
- Buyer: 10 points
- Seller: 10 points (from receiving payment)
- Referrer: 1 point
```

**Code:**
```rust
// Direct payment through marketplace
orchestrator.process_simple_payment(
    &buyer,
    &seller,
    10000i128,           // 100 USDC
    &marketplace,        // Takes platform fee
    1000u32,             // 10% fee (1000 bps)
    "marketplace_sale_001"
);

// Seller referral payment
orchestrator.process_simple_payment(
    &seller,
    &referrer,
    1000i128,            // 10 USDC
    &referrer,
    0u32,
    "marketplace_referral_001"
);
```

### 7. Batch Refunds on Dispute Timeout

**Scenario:** Multiple disputed transactions auto-refund after timeout

**Flow:**
```
Buyer 1: Dispute "order_001" (filed at ledger 1000)
Buyer 2: Dispute "order_002" (filed at ledger 1100)
Buyer 3: Dispute "order_003" (filed at ledger 1200)

Current ledger: 3100 (all disputes past resolution window)

Automated Process:
    ↓
Check all active disputes
    ├→ order_001: Resolvable? Yes (1000 + 2000 < 3100)
    │   └→ Refund to Buyer 1
    ├→ order_002: Resolvable? Yes (1100 + 2000 < 3100)
    │   └→ Refund to Buyer 2
    └→ order_003: Resolvable? Yes (1200 + 2000 < 3100)
        └→ Refund to Buyer 3

All refunds auto-processed
```

**Code:**
```rust
let disputes = vec!["order_001", "order_002", "order_003"];

for dispute_id in disputes {
    if dispute_resolver.is_resolvable(&dispute_id) {
        dispute_resolver.refund_on_timeout(&dispute_id);
    }
}
```

## Key Patterns

### Authorization Pattern
All sensitive operations require `require_auth()`:
```rust
function(&caller);
// Internally calls: caller.require_auth();
```

### Fee Calculation Pattern
Fees calculated in basis points (1 bp = 0.01%):
```rust
let fee = (amount * fee_bps / 10000) as i128;
```

### Event Emission Pattern
Track all operations:
```rust
e.events().publish((name,), (field1, field2, field3));
```

### Cross-Contract Call Pattern
Invoke other contracts with type-safe clients:
```rust
let client = ContractClient::new(&env, &address);
client.method(&arg1, &arg2);
```

## Testing These Examples

See [INTEGRATION_GUIDE.md](./INTEGRATION_GUIDE.md) for testing instructions.

## Performance Notes

- **Gas Cost**: Varies by operation, typically 1-10 Stroops per operation
- **Throughput**: Limited by Stellar ledger confirmation time (~5 seconds)
- **Batching**: Group similar operations to reduce overhead
- **Caching**: Store hot data in contract storage, not chain state

## Security Considerations

1. **Always verify amounts** before processing
2. **Check authorization** at contract entry points
3. **Avoid reentrancy** by completing state changes before external calls
4. **Validate addresses** before using in cross-contract calls
5. **Test fee calculations** carefully to avoid rounding errors
