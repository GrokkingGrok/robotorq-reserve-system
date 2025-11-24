# Wallet Implementation Status - Current Reality

**Version**: 1.0 (Actual Implementation)
**Date**: November 24, 2025
**Status**: Implemented and Working
**Platform**: Rust-based Vault System
**Note**: This document has been updated to reflect the actual implemented vault system, not the original planned Go wallet service.

---

## 🎯 **Current Reality: Vault System is the Wallet Backend**

### **What Actually Exists** (November 2025):
The **Vault System** (Rust) serves as the wallet backend and is **fully implemented and working**:

✅ **Certificate Storage**: ShadowCertVault stores RoboTorqCertificates
✅ **Stake Management**: ShadowStakeVault handles RoboStake reserves
✅ **UBD Distribution**: ShadowDistoVault distributes value via packages
✅ **User Vaults**: ShortVaultRegistry manages user landing zones
✅ **Contract Approval**: Optional service for robotic labor contracts
✅ **NATS Integration**: Full event-driven communication
✅ **Persistence**: PostgreSQL storage for schedules and recovery
✅ **Package Delivery**: UBDDistributionPackage and DemurrageReleasePackage
✅ **Simulation Features**: Time compression, drip algorithms
✅ **End-to-End UBD Pipeline**: Mint → Vault → Wallets working

### **What Was Planned But Doesn't Exist**:
❌ **Separate Go Wallet Service**: Never implemented - vault system handles this
❌ **HTTP API for Wallets**: Vault has internal APIs but no external wallet service
❌ **PostgreSQL Wallet Database**: Vault uses persistence but for schedules, not wallets
❌ **BidNet Integration**: Not yet implemented in vault
❌ **P2P Marketplace**: Not implemented
❌ **Physical RoboTorq Activation**: Not implemented

---

## 🏛️ **Actual Wallet Architecture: Phone App + Vault Backend**

### **Phone App (Future - Not Yet Implemented)**:
The phone wallet would be a **React Native app** that:

1. **Key Management**: Self-custody with biometric unlock
2. **NATS Direct Connection**: Subscribe to vault events directly
3. **Local Balance**: Track RT balance locally with NATS replay backup
4. **Package Reception**: Receive UBDDistributionPackage from vault
5. **Demurrage Calculation**: Local calculation for UX (settle to vault daily)
6. **P2P Payments**: NFC/QR payments via NATS events
7. **Vault Integration**: Transfer to/from ShortVaults via demurrage requests

**Critical**: Phone connects directly to NATS, not through HTTP API.

### **Vault Backend (Implemented)**:
The vault system provides:

1. **UBD Distribution**: Sends packages to wallet NATS topics
2. **ShortVault Management**: Demurrage-free reserves for users
3. **Certificate Authorization**: Enables UBD based on minted certificates
4. **Package Confirmation**: Receives wallet confirmations
5. **Persistence**: Survives restarts, recovers active distributions

---

## 📋 **Current Implementation Status**

### **✅ Fully Working Features**:

#### **UBD Distribution Pipeline**
- Mint creates certificates → Vault stores them
- Vault authorizes UBD → DistoVault creates distribution schedules
- DistoVault sends UBDDistributionPackage to wallets via NATS
- Wallets receive packages and credit balances
- Wallets send confirmations back to vault

#### **Package-Based Delivery**
```rust
// Actual package structure
pub struct UBDDistributionPackage {
    pub package_id: String,
    pub user_id: String,
    pub amount_canonical_jouletorq: i64,
    pub distribution_timestamp: DateTime<Utc>,
    pub provenance_cert_ids: Vec<String>,
    pub package_hash: String,
    pub vault_signature: Option<String>,
}
```

#### **Dual Delivery Modes**
- **Single Package**: Direct delivery to wallet
- **Drip-Based**: Gradual delivery to ShortVault reserves

#### **NATS Event Flow** (Actual)
```
Mint → vault.phase3.completed
    ↓
Vault stores certificates → vault.cert.stored
Vault authorizes UBD → vault.distostream.authorized
    ↓
DistoVault distributes → vault.ubd.package → Wallets
Wallets confirm → wallet.package.confirmation
```

#### **Persistence & Recovery**
- PostgreSQL stores active distribution schedules
- Vault recovers incomplete distributions on restart
- Survives network interruptions

#### **Simulation Features**
- Time compression for testing (1000x speed)
- Configurable drip algorithms (uniform, exponential, etc.)
- Economic variance factors

---

## 🚧 **Missing Components (For Full Wallet Experience)**

### **Phone App (Not Implemented)**
The actual wallet functionality requires a phone app that:

1. **Receives UBD Packages**:
   ```typescript
   // Subscribe to vault packages
   nats.subscribe('vault.ubd.package.wallet-123', (package) => {
     // Credit local balance
     balance += package.amount_canonical_jouletorq / 3_600_000;
     // Send confirmation
     nats.publish('wallet.package.confirmation', {
       package_id: package.package_id,
       wallet_id: 'wallet-123',
       received_at: new Date()
     });
   });
   ```

2. **Manages Demurrage**:
   ```typescript
   // Calculate local demurrage for UX
   const demurrageRate = getDemurrageRate(balance);
   const dailyDrain = balance * demurrageRate;

   // Send daily settlement to vault
   nats.publish('wallet.demurrage.settlement', {
     wallet_id: 'wallet-123',
     amount_canonical_jouletorq: dailyDrain * 3_600_000,
     settlement_date: new Date()
   });
   ```

3. **Handles ShortVault Transfers**:
   ```typescript
   // Request demurrage release from ShortVault
   nats.publish('wallet.demurrage.request', {
     wallet_id: 'wallet-123',
     short_vault_id: 'short-vault-456',
     request_amount: 1000 * 3_600_000
   });

   // Receive response
   nats.subscribe('vault.demurrage.package', (package) => {
     balance += package.amount_canonical_jouletorq / 3_600_000;
   });
   ```

### **P2P Payments (Not Implemented)**
```typescript
// Send payment
const payment = {
  payment_id: uuid(),
  from_wallet: 'wallet-123',
  to_wallet: 'wallet-456',
  amount_canonical_jouletorq: 10 * 3_600_000,
  timestamp: new Date()
};

// Sign with private key
payment.signature = sign(payment, privateKey);

// Publish to network
nats.publish('payment.sent', payment);

// Deduct local balance immediately
balance -= 10;
```

### **Vault Integration (Partially Implemented)**
- ShortVault creation: ✅ Working
- UBD crediting to ShortVaults: ✅ Working
- Demurrage release requests: ✅ Working
- TorqedPledge management: ❌ Not implemented

---

## 🔄 **Architecture Evolution**

### **Original Plan (Outdated)**
- Separate Go wallet service with HTTP API
- PostgreSQL database for wallet balances
- RESTful API for phone app to call
- BidNet integration for investments

### **Current Reality (Implemented)**
- Rust vault system handles wallet backend functions
- NATS-only communication (no HTTP API for wallets)
- Phone apps connect directly to NATS
- Package-based delivery instead of balance updates
- Persistence for distribution schedules, not wallet balances

### **Key Changes**
1. **No separate wallet service** - vault system IS the wallet backend
2. **NATS-centric** - all communication via pub/sub, no REST APIs
3. **Package delivery** - wallets receive delivery packages, not balance updates
4. **Local balance management** - phones track balances locally with NATS backup
5. **Event sourcing** - wallet state reconstructable from NATS event replay

---

## 🎯 **Next Steps for Complete Wallet Experience**

### **Immediate (Wallet MVP)**
1. **Phone App Development**:
   - React Native app with NATS client
   - Local SQLite encrypted database
   - Key management with biometric unlock
   - UBD package reception and confirmation
   - Local balance tracking with demurrage calculation

2. **P2P Payment System**:
   - NFC/QR code payment interface
   - NATS-based payment events
   - Instant balance updates
   - Transaction history

3. **ShortVault Integration**:
   - View ShortVault balances
   - Request demurrage releases
   - Auto-save configurations

### **Future Enhancements**
1. **TorqedPledge Management**: Create and track pledges
2. **Investment Portal**: BidNet contract browsing and investment
3. **P2P Marketplace**: Goods/services trading
4. **Currency Exchange**: RT ↔ USDC via BidNet
5. **Physical RoboTorq Activation**: NFC scanning for wallet funding

---

## 📊 **Current System Capabilities**

### **✅ Working End-to-End**
- Certificate minting and storage
- UBD authorization based on certificates
- Package creation and delivery
- Distribution schedule persistence
- Recovery after restarts
- Simulation testing features

### **🔄 Integration Points**
- **Mint**: Sends Phase3 completion events
- **Wallets**: Receive UBD packages (when implemented)
- **NATS**: Message bus for all communication
- **PostgreSQL**: Schedule persistence

### **🎯 Production Ready**
- Comprehensive error handling
- Atomic operations for concurrency
- Monitoring and metrics
- Docker deployment
- Health checks

---

## 🏁 **Conclusion**

The **vault system is implemented and working** as the wallet backend. The missing piece is the **phone app** that connects to NATS and manages local wallet state. The original plan for a separate Go wallet service was never implemented - the Rust vault system serves this purpose.

**Current Status**: Backend ready, frontend needed for complete wallet experience.

---

*"The vault system delivers UBD packages to wallets - now we need the wallets to receive them."*### **Phase 2 - Vaults & Demurrage**:
**What will exist**: Vault Service (StashVault + TorqedPledge)
**What's needed**: Savings integration, demurrage mechanics

**Additional Features**:
8. **Demurrage Calculation**: Calculate continuous demurrage (tiered, settled daily)
9. **Demurrage Settlement**: Publish daily batch to Vault Service
10. **StashVault Integration**: Transfer RT to/from StashVaults
11. **TorqedPledge Integration**: Create pledges, track progress
12. **Vault Dashboard**: View yields, balances, progress
13. **Auto-Save Configuration**: Route % of UBD to vaults at DistoDam

**Goal**: Users can protect RT from demurrage and save for big purchases

---

### **Phase 3 - Investment & Marketplace**:
**What will exist**: BidNet (contract evaluation), Trust (BRLA execution)
**What's needed**: Investment UI, peer trading, currency exchange

**Additional Features**:
14. **Investment Portal**: Browse BidNet contracts, invest in projects
15. **Contract Dashboard**: Track active investments, ROI, returns
16. **P2P Marketplace**: List/buy goods & services (small trades)
17. **Currency Exchange**: Large RT ↔ USDC trades via BidNet (>500 RT)
18. **Rating System**: Buyer/seller reputation
19. **Escrow Payments**: Optional for large purchases

**Goal**: Users can invest in production, trade peer-to-peer, and exchange currencies

---

### **Phase 4 - Education & Onboarding**:
**What will exist**: Physical RoboTorq coins (manufacturing complete)
**What's needed**: NFC scanning, tutorial system

**Additional Features**:
20. **Physical RoboTorq Activation**: NFC scan to fund wallet
21. **Tutorial System**: Interactive guides for new users
22. **Sandbox Mode**: Practice with fake RT
23. **Progressive Disclosure**: Unlock features as user learns
24. **Context-Sensitive Help**: Tooltips, hints throughout app

**Goal**: Smooth onboarding for non-technical users

---

### **Phase 5 - Builder Tools**:
**What will exist**: Robot network growing, builder demand
**What's needed**: Tools for builders to coordinate production

**Additional Features**:
25. **Bid Posting**: Submit projects to BidNet
26. **Robot Listing**: Offer robotic services to network
27. **Fleet Dashboard**: Manage multiple robots
28. **Contract Management**: Milestones, payments
29. **Supplier/Distributor Listings**: Materials & logistics

**Goal**: Enable builders to coordinate robotic production

---

### **Phase 6 - Advanced Features**:
**What will exist**: Network maturity, consensus needs
**What's needed**: Decentralized verification, advanced DeFi

**Additional Features**:
30. **Proof Verification**: Validate proofs, earn fees
31. **Multi-Device Sync**: Same wallet on multiple phones
32. **Social Recovery**: Trusted contacts help recover wallet
33. **Unified Vault Slider**: Visual Safe ←→ Build allocation
34. **L2 Integration**: Blockchain settlement, smart contracts

**Goal**: Mature decentralized economy with advanced features

---

### **Key Architectural Principle**:
**The phone IS the wallet** - there is no separate "Wallet Service" backend. The phone:
- Subscribes to NATS topics directly
- Manages balance locally (encrypted database)
- Publishes spending/demurrage events to NATS
- Can reconstruct state from NATS event replay if needed

---

## 🏛️ **Strategic Vision: Phone Wallet as Economic Interface**

### **Long-Term Role of Wallet App**:
The Phone Wallet is the **primary interface** between humans and the RoboTorq economy. It serves multiple functions:

#### **1. Onboarding Gateway** (Future - Physical RoboTorq Activation)
- **Entry Requirement**: Users must scan/deposit physical RoboTorq coins to activate wallet
- **NFC Integration**: Tap physical RT coins to phone to activate
- **Minimum Threshold**: Configurable amount (e.g., 100 RT to start)
- **Learning Mechanism**: Real-world trade forces users to understand RT value before entering digital economy
- **Purpose**: Prevents Sybil attacks, ensures skin-in-the-game, teaches value anchoring

#### **2. UBD Mining Display** (MVP - Core UX Feature)
- **Batch Reception**: Receive daily UBD batch from DistoDam at midnight
- **Continuous Mining**: Display smooth balance increase over 24 hours (configurable curves)
- **Mining Curves**: User chooses distribution pattern:
  - **Uniform**: Steady flow (75 RT / 86400 seconds)
  - **Normal Distribution**: Peak at configured time (lunch, dinner)
  - **Weibull**: Custom peak (e.g., 6pm happy hour)
  - **Front-loaded**: Most RT available early morning
  - **Back-loaded**: Most RT available evening/night
- **Visual Feedback**: Animated coin "mining" into wallet
- **Purpose**: Psychologically satisfying UX, user gets RT when they need it

#### **3. Demurrage Awareness** (MVP - Transparency)
- **Continuous Calculation**: Display demurrage draining idle balance in real-time
- **Tiered Visualization**: Show which tier user is in (no penalty, 1%, 2%, 3%)
- **Ratio Display**: "You're holding 15x your daily UBD - consider vaulting!"
- **Vault Suggestions**: Prompt to move excess RT to StashVault
- **Purpose**: Educate users on demurrage mechanics, encourage healthy vault ratio

#### **4. Education Platform** (MVP - Guided Onboarding)
- **Tutorial System**: Step-by-step guides for new users
  - How to receive your daily UBD (mining visualization)
  - How to buy goods from other wallet holders (P2P marketplace)
  - How to invest in BidNet projects (passive income)
  - How to save in vaults (demurrage protection)
  - How to verify proofs-of-work (earn fees for validation - future)
- **Progressive Disclosure**: Features unlock as users gain experience
- **Interactive Tutorials**: Sandbox mode with fake RT to practice
- **Tooltips & Hints**: Context-sensitive help throughout app
- **Purpose**: Onboard users smoothly, teach economic mechanics

#### **5. Investment Portal** (Phase 3 - BidNet Integration)
- **Contract Browsing**: View available production contracts awaiting funding
- **Investment Dashboard**: Track active investments, ROI, returns
- **Risk Metrics**: Risk scores, capability scores (when available)
- **TorqedPledge Management**: Track progress toward house/car/fleet purchase
- **Stake Management**: Lock RT in contracts, receive returns on completion
- **Purpose**: Passive income opportunities, long-term wealth building

#### **6. P2P Marketplace** (Phase 3 - Peer Trading - Small Trades)
- **Goods Listings**: Browse items for sale from other users
- **Service Listings**: Find local services (tutoring, repairs, rides)
- **Seller Profile**: Post your own goods/services
- **Rating System**: Reputation scores for buyers/sellers
- **NFC/QR Payments**: Seamless in-person transactions
- **Escrow**: Local wallet escrow for trades
- **Trade Limits**: Best for <500 RT trades (use Currency Exchange for larger)
- **Purpose**: Real-world economic activity, merchant adoption

#### **6b. Currency Exchange** (Phase 3 - Large RT ↔ USDC Trades via BidNet)
- **Exchange Interface**: "I want to sell 10,000 RT for USDC"
- **Market Rate Display**: Current RT/USDC rate (live from BidNet)
- **Slippage Control**: Max acceptable price variance (e.g., ±2%)
- **Escrow Locking**: RT locked in wallet, USDC address provided
- **BidNet Submission**: Create currency exchange contract
- **Matching Updates**: Real-time notifications when buyer/seller found
- **Settlement Tracking**: Monitor USDC transfer, RT release
- **Trade History**: View completed exchanges, rates achieved
- **Reputation Impact**: Exchange trades affect wallet reputation
- **Limits**: 
  - Min trade: 500 RT (smaller trades use P2P Marketplace)
  - Max trade: Based on BidNet liquidity pool depth
  - Daily limit: Configurable per wallet reputation
- **Payment Methods**:
  - USDC-Ethereum (wallet → wallet)
  - USDC-Polygon (lower gas fees)
  - Coinbase (email transfer)
  - Wire Transfer (for very large trades >$10k)
- **Purpose**: Large-scale RT ↔ fiat conversion without leaving RoboTorq network

**Exchange Flow**:
```
1. User: "Sell 10,000 RT for USDC"
   - Wallet locks 10,000 RT (local escrow)
   - User enters USDC receiving address (Ethereum/Polygon)
   
2. Wallet → BidNet: Submit currency exchange contract
   PUBLISH contracts.submitted {
     "type": "currency_exchange",
     "exchange": {
       "from_currency": "RT",
       "to_currency": "USDC",
       "from_amount": 10000.0,
       "to_amount_min": 1800.0,  // Min acceptable (slippage)
       "price_per_rt": 0.18,
       "payment_methods": ["USDC-Ethereum"],
       "settlement_deadline": "2025-11-20T00:00:00Z"
     },
     "escrow": {
       "rt_locked": 10000.0,
       "locked_at": "2025-11-15T12:00:00Z"
     }
   }
   
3. BidNet evaluates contract (rate check, liquidity, reputation)
   
4. BidNet matches with buyer OR fills from liquidity pool
   
5. Wallet receives notification:
   SUBSCRIBE contracts.matched.{wallet_id}
   - Buyer found: Bob (4.9★ rating)
   - Trade amount: 10,000 RT → 1,820 USDC
   - Settlement: Bob sends USDC → your address
   
6. User confirms USDC received (checks Coinbase/MetaMask)
   
7. Wallet releases RT escrow:
   PUBLISH payment.sent {
     "to_wallet_id": "wallet-bob",
     "amount_rt": 10000.0,
     "reason": "currency_exchange"
   }
   
8. Both parties rate each other
   - Builds reputation for future trades
```

#### **7. Builder Portal** (Phase 5 - Desktop/Web Companion App)
- **Bid Posting**: Submit projects for robotic labor (Builder → BidNet)
- **Robot Listing**: Offer robotic services to the network
- **Distribution Services**: List logistics/delivery capabilities
- **Supplier Services**: Offer materials/components to builders
- **Contract Management**: Monitor active contracts, milestones, payments
- **Fleet Dashboard**: Manage multiple robots, track utilization
- **Purpose**: Enable production, coordinate robot economy

#### **8. P2P Payments** (MVP - NFC/QR)
- **NFC Tap-to-Pay**: Tap phones together to send/receive RT
- **QR Code Payment**: Scan merchant QR for payment
- **Payment Confirmation**: Instant feedback with haptic/sound
- **Transaction Signing**: Local private key signs all transactions
- **NATS Publishing**: Broadcast `payment.sent` to network
- **Split Payments**: Pay multiple people at once (dinner splitting)
- **Purpose**: Real-world spending, merchant adoption, network effects

#### **9. Vault Management** (MVP - Savings Integration)
- **StashVault**: View balance, deposit, withdraw (instant liquidity)
- **TorqedPledge**: Create pledge, track progress, view maturity
- **Yield Dashboard**: See demurrage earnings, pledge yields
- **Auto-Save Configuration**: "Save 20% of UBD automatically"
- **Vault Slider**: Visual control (Safe ←→ Build allocation)
- **Purpose**: Demurrage protection, long-term savings, big purchases

#### **10. DistoDam Subscription** (Phase 1 - REQUIRED FOR MVP)
- **UBD Enrollment**: Subscribe to DistoDam for Universal Basic Distribution
- **Subscription Fee**: Pay initial & recurring fees (if applicable)
- **Purpose**: **FIRST STEP** - Without this, no UBD batches will be sent to wallet!
- **Note**: This MUST work before any other wallet features can be tested

#### **10b. DistoDam Configuration** (Phase 2 - Advanced UBD Management)
- **DistoStream Auto-Routing**: Configure pre-diversion (wallet %, vault %, investment %)
- **Batch History**: View all UBD batches received
- **Projections**: "At this rate, you'll have X RT by Y date"
- **Purpose**: Optimize UBD allocation

#### **11. Proof Verification** (Future - Consensus Participation)
- **Backlog Processing**: When proof verification backlogs occur, wallets can help process
- **Verification Rewards**: Earn fees for validating proofs-of-work
- **Consensus Participation**: Help maintain network integrity
- **Gamification**: Leaderboards for top verifiers
- **Purpose**: Decentralized consensus, network security, earn extra RT

#### **12. Key Management** (MVP - Security)
- **Private Key Generation**: Create wallet on first launch
- **Biometric Protection**: FaceID/TouchID/fingerprint unlock
- **Backup & Recovery**: Seed phrase backup, cloud recovery (encrypted)
- **Multi-Device Sync**: Same wallet on multiple phones (optional)
- **Social Recovery**: Trusted contacts can help recover wallet (future)
- **Purpose**: Self-custody, security, user control

---

## 🏗️ **Application Architecture**

### **Technology Stack**:
- **Frontend**: React Native (iOS + Android from one codebase)
- **Local Database**: SQLite (encrypted with SQLCipher)
- **Messaging**: NATS client library (WebSocket connection)
- **Cryptography**: libsodium (Ed25519 signatures, encryption)
- **State Management**: Redux + Redux Persist
- **UI Framework**: React Native Paper (Material Design)

### **Design Principles**:
1. **Offline-First**: Phone must work without constant internet (queue transactions)
2. **Event Sourcing**: Balance reconstructable from NATS event replay
3. **Client-Side Economics**: Mining curves, demurrage calculation happen locally (UX)
4. **Security-First**: Private keys never leave device
5. **Simple UX**: Grandma-friendly interface, progressive disclosure

---

### **Core Components**:

### **1. MiningEngine**
**Responsibility**: Simulate continuous UBD "mining" from daily batch

**Features**:
- Configurable distribution curves (uniform, normal, Weibull)
- Real-time balance updates (visual feedback)
- Smooth interpolation over 24 hours
- Reset at midnight when new batch arrives

**State**:
```typescript
interface MiningState {
  dailyBatch: number;           // RT received at midnight
  minedSoFar: number;           // RT mined since midnight
  miningCurve: CurveConfig;     // User's chosen curve
  startTime: Date;              // When batch was received
  lastUpdateTime: Date;         // For interpolation
}
```

---

### **2. DemurrageCalculator**
**Responsibility**: Calculate continuous demurrage on phone balance

**Features**:
- Tiered demurrage based on balance/daily_disto ratio
- Continuous compounding display
- Daily settlement (publish to NATS)
- Visual warnings when approaching higher tiers

**Tiers** (configurable):
```typescript
interface DemurrageTiers {
  tier0: { threshold: 0,   rate: 0.00 },  // 0-10x daily disto
  tier1: { threshold: 10,  rate: 0.01 },  // 10-50x
  tier2: { threshold: 50,  rate: 0.02 },  // 50-100x
  tier3: { threshold: 100, rate: 0.03 },  // >100x
}
```

**Settlement**:
- Runs daily at midnight
- Publishes `demurrage.paid` to NATS
- Vault Service aggregates and distributes

---

### **3. BalanceManager**
**Responsibility**: Track spendable RT balance locally

**Features**:
- Local SQLite storage (encrypted)
- NATS event replay for recovery
- Transaction history
- Spending limits (optional, user-configurable)

**Balance Calculation**:
```typescript
balance = 
  Sum(ubd.funded received) 
  + Sum(vault.withdrawn)
  + mining.minedSoFar
  - Sum(payment.sent)
  - Sum(phone.vault_transfer deposits)
  - demurrage.accumulatedToday
```

---

### **4. PaymentProcessor**
**Responsibility**: Handle NFC/QR payments

**Features**:
- NFC peer-to-peer (phone-to-phone)
- QR code generation (receive)
- QR code scanning (send)
- Transaction signing (Ed25519)
- Publish `payment.sent` to NATS
- Instant visual feedback

**Payment Flow**:
1. User taps "Pay 5 RT"
2. Phone deducts from local balance
3. Phone signs transaction with private key
4. Phone publishes `payment.sent` to NATS
5. Merchant phone receives event
6. Merchant phone credits balance
7. Both phones show confirmation

---

### **5. VaultConnector**
**Responsibility**: Interface with Vault Service

**Features**:
- View StashVault balance (subscribe to `vault.deposited`)
- Deposit to vault (publish `phone.vault_transfer`)
- Withdraw from vault (publish `phone.vault_transfer`)
- View TorqedPledge progress
- Create new pledges
- Track demurrage yields

**NATS Topics**:
- Subscribe: `vault.deposited`, `vault.withdrawn`, `vault.pledge_ready`
- Publish: `phone.vault_transfer`

---

### **5b. ExchangeConnector** (Phase 3 - Currency Exchange via BidNet)
**Responsibility**: Interface with BidNet for large RT ↔ USDC trades

**Features**:
- Submit currency exchange contracts to BidNet
- Lock RT in local wallet escrow during pending trade
- Monitor exchange contract status (matching, settlement)
- Release RT when USDC confirmed received
- Track exchange history
- Display current market rates

**NATS Topics**:
- Subscribe: `contracts.matched.{walletID}`, `contracts.rejected.{walletID}`
- Publish: `contracts.submitted` (to BidNet)

**Exchange Flow**:
```typescript
class ExchangeConnector {
  async submitExchange(
    fromCurrency: "RT" | "USDC",
    toCurrency: "RT" | "USDC",
    fromAmount: number,
    toAmountMin: number,
    usdcAddress: string,
    paymentMethods: string[]
  ): Promise<string> {
    // 1. Lock RT in local escrow
    const contractId = `contract-ex-${uuid()}`;
    this.balanceManager.lockEscrow(fromAmount, contractId);
    
    // 2. Create currency exchange contract
    const contract = {
      id: contractId,
      type: "currency_exchange",
      builder: this.walletId,  // Wallet ID as submitter
      status: "pending",
      exchange: {
        from_currency: fromCurrency,
        to_currency: toCurrency,
        from_amount: fromAmount,
        to_amount_min: toAmountMin,
        price_per_unit: toAmountMin / fromAmount,
        payment_methods: paymentMethods,
        settlement_deadline: new Date(Date.now() + 5*24*60*60*1000) // 5 days
      },
      escrow: {
        rt_locked: fromCurrency === "RT" ? fromAmount : 0,
        usdc_deposited: fromCurrency === "USDC" ? fromAmount : 0,
        locked_at: new Date()
      },
      created_at: new Date()
    };
    
    // 3. Publish to BidNet
    await this.natsClient.publish('contracts.submitted', contract);
    
    // 4. Store in local database
    await this.db.saveExchangeContract(contract);
    
    return contractId;
  }
  
  async handleMatched(event: ContractMatchedEvent) {
    // BidNet found buyer/seller match
    const contract = await this.db.getExchangeContract(event.contractId);
    
    // Update UI: "Match found! Buyer: Bob (4.9★)"
    this.ui.showNotification({
      title: "Exchange Matched!",
      message: `Matched with ${event.counterparty.name} (${event.counterparty.rating}★)`,
      action: "View Details"
    });
    
    // Wait for user to confirm USDC received
    // Then release RT escrow
  }
  
  async confirmUSDCReceived(contractId: string) {
    // User checked Coinbase/MetaMask and confirms USDC arrived
    const contract = await this.db.getExchangeContract(contractId);
    
    // Release RT from escrow to buyer
    await this.paymentProcessor.sendPayment(
      contract.matched_buyer_id,
      contract.exchange.from_amount,
      "currency_exchange"
    );
    
    // Unlock escrow locally
    await this.balanceManager.releaseEscrow(contractId);
    
    // Mark complete
    contract.status = "completed";
    await this.db.updateExchangeContract(contract);
    
    // Prompt for rating
    this.ui.showRatingPrompt(contract.matched_buyer_id);
  }
  
  async cancelExchange(contractId: string) {
    // User cancels before matching
    const contract = await this.db.getExchangeContract(contractId);
    
    // Release escrow
    await this.balanceManager.releaseEscrow(contractId);
    
    // Publish cancellation to BidNet
    await this.natsClient.publish('contracts.cancelled', {
      contract_id: contractId,
      cancelled_at: new Date()
    });
    
    contract.status = "cancelled";
    await this.db.updateExchangeContract(contract);
  }
}
```

**Local Escrow Management**:
```typescript
interface EscrowedExchange {
  contractId: string;
  amountRT: number;
  lockedAt: Date;
  expiresAt: Date;  // Auto-release after 5 days
  status: "pending" | "matched" | "completed" | "cancelled";
}

class BalanceManager {
  escrowedExchanges: Map<string, EscrowedExchange> = new Map();
  
  lockEscrow(amount: number, contractId: string) {
    // Deduct from spendable balance
    this.spendableBalance -= amount;
    
    // Add to escrow
    this.escrowedExchanges.set(contractId, {
      contractId,
      amountRT: amount,
      lockedAt: new Date(),
      expiresAt: new Date(Date.now() + 5*24*60*60*1000),
      status: "pending"
    });
    
    // Persist
    this.db.saveEscrow(contractId, amount);
  }
  
  releaseEscrow(contractId: string) {
    const escrow = this.escrowedExchanges.get(contractId);
    if (!escrow) return;
    
    // If cancelled (not completed), return to spendable
    if (escrow.status === "cancelled") {
      this.spendableBalance += escrow.amountRT;
    }
    
    // Remove from escrow
    this.escrowedExchanges.delete(contractId);
    this.db.deleteEscrow(contractId);
  }
  
  // Auto-release expired escrows (cron job)
  async releaseExpiredEscrows() {
    const now = new Date();
    for (const [contractId, escrow] of this.escrowedExchanges) {
      if (escrow.expiresAt < now && escrow.status === "pending") {
        // Settlement deadline passed, release escrow
        await this.cancelExchange(contractId);
      }
    }
  }
}
```

**Database Schema** (addition to wallet DB):
```sql
CREATE TABLE exchange_contracts (
    id              VARCHAR(64) PRIMARY KEY,
    wallet_id       VARCHAR(64) NOT NULL,
    from_currency   VARCHAR(10) NOT NULL,
    to_currency     VARCHAR(10) NOT NULL,
    from_amount     FLOAT NOT NULL,
    to_amount_min   FLOAT NOT NULL,
    price_per_unit  FLOAT NOT NULL,
    matched_buyer_id VARCHAR(64),
    matched_at      TIMESTAMP,
    completed_at    TIMESTAMP,
    cancelled_at    TIMESTAMP,
    status          VARCHAR(20) NOT NULL,  -- pending, matched, completed, cancelled, rejected
    rejection_reason VARCHAR(255),
    created_at      TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE exchange_escrows (
    contract_id     VARCHAR(64) PRIMARY KEY REFERENCES exchange_contracts(id),
    amount_rt       FLOAT NOT NULL,
    locked_at       TIMESTAMP NOT NULL,
    expires_at      TIMESTAMP NOT NULL,
    released_at     TIMESTAMP
);
```

---

### **6. KeyManager**
**Responsibility**: Secure private key storage and cryptographic operations

**Features**:
- Generate Ed25519 keypair on first launch
- Store private key in iOS Keychain / Android Keystore
- Sign transactions
- Encrypt local database
- Seed phrase backup (BIP39)
- Biometric unlock

**Security**:
- Private key NEVER leaves device
- All transactions signed locally
- Local database encrypted with key derived from user PIN + device ID

---

### **7. NATSClient**
**Responsibility**: NATS connection and event handling

**Features**:
- WebSocket connection to NATS server
- Subscribe to user-specific topics
- Publish events (payments, demurrage)
- Event replay for recovery
- Reconnect on network loss
- Queue transactions when offline

**Subscriptions**:
```typescript
const topics = [
  `ubd.funded.${walletID}`,                 // Daily UBD batch
  `vault.deposited.${walletID}`,            // Vault confirmations
  `vault.withdrawn.${walletID}`,            // Vault withdrawals
  `vault.pledge_ready.${walletID}`,         // TorqedPledge matured
  `payment.received.${walletID}`,           // Incoming payments
  `contracts.matched.${walletID}`,          // Exchange matched (Phase 3)
  `contracts.rejected.${walletID}`,         // Exchange rejected (Phase 3)
];
```

---

### **System Design Principles**:
1. **Modular Components**: Following Mint/BidNet pattern
2. **Headless First**: Backend service with HTTP API (UI is separate)
3. **NATS-Centric**: Pub/sub for all inter-service communication
4. **Database-Backed**: PostgreSQL for wallet state persistence
5. **Extensible**: Designed to accommodate all future features

---

### **1. Config**
**Responsibility**: Load and validate configuration from environment

**Environment Variables**:
```bash
# HTTP server
HTTP_PORT=3001              # Default: "3001" (avoid conflict with existing services)

# NATS connection
NATS_URL=nats://nats:4222   # Default: "nats://nats:4222"

# NATS topics (configurable for testing)
UBD_FUNDED_TOPIC=ubd.funded                 # Input from DistoDam
CONTRACTS_APPROVED_TOPIC=contracts.approved # Input from BidNet (future)
WALLET_INVESTMENTS_TOPIC=wallet.investments # Output to BidNet (future)

# Database connection
DB_HOST=postgres            # Default: "postgres"
DB_PORT=5432                # Default: "5432"
DB_NAME=wallet_db           # Default: "wallet_db"
DB_USER=wallet              # Default: "wallet"
DB_PASSWORD=wallet_pass     # Default: "wallet_pass"
DB_SSL_MODE=disable         # Default: "disable" (use "require" in production)

# Wallet configuration
MIN_ACTIVATION_RT=1.0       # Minimum RT to activate wallet (future)
UBD_SUBSCRIPTION_FEE=0.0    # Initial UBD subscription fee (0 for MVP)
INVESTMENT_MIN_RT=0.1       # Minimum investment amount (future)

# Metrics & monitoring
METRICS_ENABLED=true        # Default: "true"
METRICS_PORT=9094           # Prometheus metrics port

# Logging
LOG_LEVEL=info              # Options: debug, info, warn, error
LOG_FORMAT=json             # Options: json, text
```

**Interface**:
```go
type Config struct {
    // HTTP server
    HTTPPort string

    // NATS
    NATSURL              string
    UBDFundedTopic       string
    ContractsApprovedTopic string
    WalletInvestmentsTopic string

    // Database
    DBHost     string
    DBPort     string
    DBName     string
    DBUser     string
    DBPassword string
    DBSSLMode  string

    // Wallet settings
    MinActivationRT     float64
    UBDSubscriptionFee  float64
    InvestmentMinRT     float64

    // Metrics
    MetricsEnabled bool
    MetricsPort    string

    // Logging
    LogLevel  string
    LogFormat string
}

func LoadConfig() (*Config, error)
func (c *Config) Validate() error
func (c *Config) GetDBConnectionString() string
```

**Validation Rules**:
- `HTTPPort` must be valid port number
- `NATSURL` must be valid NATS connection string
- `DBHost`, `DBName`, `DBUser` must be non-empty
- `MinActivationRT` >= 0.0
- `LogLevel` must be one of: debug, info, warn, error

**Tests** (29 tests):
- ✅ Load from environment variables (8 tests)
- ✅ Default values when env vars missing (6 tests)
- ✅ Validation errors (8 tests)
- ✅ Database connection string generation (3 tests)
- ✅ Invalid port numbers (2 tests)
- ✅ Invalid NATS URLs (2 tests)

---

### **2. UBDReceiver**
**Responsibility**: Subscribe to `ubd.funded` topic, update wallet balances

**NATS Subscription**: `ubd.funded` (from DistoDam)

**Input Message Structure**:
```go
type UBDFundedEvent struct {
    RequestID   string    `json:"request_id"`    // UUID from DistoDam
    WalletID    string    `json:"wallet_id"`     // Target wallet
    Amount      float64   `json:"amount"`        // RT funded
    FundedAt    time.Time `json:"funded_at"`     // UTC timestamp
    FundedBy    string    `json:"funded_by"`     // "distodam-001"
    TxHash      string    `json:"tx_hash"`       // Future: blockchain tx (optional)
}
```

**Interface**:
```go
type UBDReceiver interface {
    Start(ctx context.Context) error
    Stop() error
    GetStats() UBDReceiverStats
}

type UBDReceiverStats struct {
    TotalReceived   uint64  // Total UBD events received
    TotalAmountRT   float64 // Total RT received
    LastEventAt     time.Time
    ProcessingErrors uint64
}
```

**Behavior**:
1. Subscribe to `ubd.funded` topic
2. Parse `UBDFundedEvent` message
3. Validate:
   - `WalletID` exists in database
   - `Amount` > 0
   - `RequestID` not already processed (idempotency)
4. Update wallet balance atomically: `balance += Amount`
5. Create transaction record: type="ubd_inflow"
6. Acknowledge NATS message
7. Update metrics (total received, processing time)

**Error Handling**:
- Invalid JSON → log error, NACK message (retry)
- Wallet not found → log warning, ACK message (DistoDam may be ahead)
- Duplicate `RequestID` → log info, ACK message (idempotent)
- Database error → log error, NACK message (retry with backoff)

**Tests** (18 tests):
- ✅ Successful UBD event processing (3 tests)
- ✅ Wallet balance update (atomic operation) (4 tests)
- ✅ Transaction record creation (3 tests)
- ✅ Invalid JSON handling (2 tests)
- ✅ Wallet not found handling (2 tests)
- ✅ Duplicate RequestID (idempotency) (2 tests)
- ✅ Database errors with retry (2 tests)

---

### **3. BalanceManager**
**Responsibility**: Manage wallet balances with atomic operations

**Interface**:
```go
type BalanceManager interface {
    // Core operations
    GetBalance(ctx context.Context, walletID string) (float64, error)
    Credit(ctx context.Context, walletID string, amount float64, reason string) error
    Debit(ctx context.Context, walletID string, amount float64, reason string) error
    
    // Atomic operations
    Transfer(ctx context.Context, fromWalletID, toWalletID string, amount float64, reason string) error
    
    // Balance checks
    HasSufficientBalance(ctx context.Context, walletID string, amount float64) (bool, error)
    
    // Stats
    GetStats() BalanceManagerStats
}

type BalanceManagerStats struct {
    TotalCredits    uint64
    TotalDebits     uint64
    TotalTransfers  uint64
    FailedOps       uint64
}
```

**Behavior**:
- **Credit**: Add to wallet balance (UBD inflows, investment returns)
- **Debit**: Subtract from wallet balance (investments, subscription fees)
- **Transfer**: Atomic transfer between two wallets (P2P payments - future)
- **Atomicity**: Use database transactions for all operations
- **Validation**: Amount > 0, wallet exists, sufficient balance for debits

**Micro-RT Precision**:
- Store balances as `INT64` in micro-RT (1 RT = 1,000,000 µRT)
- Convert to/from `float64` RT at API boundaries
- Prevents floating-point precision errors

**Tests** (22 tests):
- ✅ Credit wallet balance (4 tests)
- ✅ Debit wallet balance (4 tests)
- ✅ Insufficient balance errors (3 tests)
- ✅ Atomic transfer (2 wallets updated) (4 tests)
- ✅ Transfer failures rollback (3 tests)
- ✅ Concurrent operations (race conditions) (4 tests)

---

### **4. TransactionLogger**
**Responsibility**: Record all wallet transactions for audit trail

**Interface**:
```go
type TransactionLogger interface {
    // Log transaction
    LogTransaction(ctx context.Context, tx *Transaction) error
    
    // Query transactions
    GetTransactionsByWallet(ctx context.Context, walletID string, limit, offset int) ([]*Transaction, error)
    GetTransactionByID(ctx context.Context, txID string) (*Transaction, error)
    
    // Stats
    GetStats() TransactionLoggerStats
}

type Transaction struct {
    ID          string    `json:"id"`           // UUID
    WalletID    string    `json:"wallet_id"`    // Primary wallet
    Type        string    `json:"type"`         // "ubd_inflow", "investment", "subscription", "transfer_in", "transfer_out"
    Amount      float64   `json:"amount"`       // RT amount (positive for inflows, negative for outflows)
    BalanceAfter float64  `json:"balance_after"` // Wallet balance after transaction
    RelatedTo   string    `json:"related_to"`   // RequestID, ContractID, or other reference
    Metadata    string    `json:"metadata"`     // JSON blob for additional data
    CreatedAt   time.Time `json:"created_at"`   // UTC timestamp
}

type TransactionLoggerStats struct {
    TotalTransactions uint64
    LogErrors         uint64
}
```

**Transaction Types**:
- `ubd_inflow`: UBD funding from DistoDam
- `investment`: Investment in BidNet contract
- `investment_return`: Returns from successful contract
- `subscription`: DistoDam subscription fee
- `transfer_in`: Received from another wallet (future)
- `transfer_out`: Sent to another wallet (future)
- `activation`: Physical RoboTorq deposit (future)

**Tests** (15 tests):
- ✅ Log transaction successfully (3 tests)
- ✅ Query transactions by wallet (pagination) (4 tests)
- ✅ Query transaction by ID (2 tests)
- ✅ Transaction types validation (3 tests)
- ✅ Database errors handling (3 tests)

---

### **5. APIServer**
**Responsibility**: Expose HTTP endpoints for wallet operations

**Endpoints**:

#### **Wallet Management**
```
POST   /api/v1/wallet                    Create new wallet
GET    /api/v1/wallet/:id                Get wallet details
GET    /api/v1/wallet/:id/balance        Get wallet balance
GET    /api/v1/wallet/:id/transactions   Get transaction history (paginated)
```

#### **DistoDam Integration** (MVP)
```
POST   /api/v1/wallet/:id/subscribe      Subscribe to DistoDam for UBD
GET    /api/v1/wallet/:id/ubd-status     Get UBD subscription status
```

#### **BidNet Integration** (Future - Phase 2)
```
GET    /api/v1/contracts                 List available contracts
GET    /api/v1/contracts/:id             Get contract details
POST   /api/v1/wallet/:id/invest         Invest in contract
GET    /api/v1/wallet/:id/investments    List wallet's investments
```

#### **Health & Metrics**
```
GET    /health                           Health check
GET    /metrics                          Prometheus metrics
```

**Request/Response Examples**:

**Create Wallet**:
```http
POST /api/v1/wallet
Content-Type: application/json

{
  "owner_id": "user-123",          // Future: link to identity system
  "activation_amount": 1.5,        // Future: physical RT deposit
  "metadata": {
    "name": "My Primary Wallet",
    "created_via": "web_app"
  }
}

Response 201:
{
  "id": "wallet-abc123",
  "owner_id": "user-123",
  "balance": 0.0,
  "status": "active",
  "created_at": "2025-11-14T12:00:00Z"
}
```

**Get Balance**:
```http
GET /api/v1/wallet/wallet-abc123/balance

Response 200:
{
  "wallet_id": "wallet-abc123",
  "balance": 42.5,
  "last_updated": "2025-11-14T12:30:00Z"
}
```

**Get Transactions**:
```http
GET /api/v1/wallet/wallet-abc123/transactions?limit=10&offset=0

Response 200:
{
  "wallet_id": "wallet-abc123",
  "transactions": [
    {
      "id": "tx-001",
      "type": "ubd_inflow",
      "amount": 2.5,
      "balance_after": 42.5,
      "related_to": "ubd-req-456",
      "created_at": "2025-11-14T12:30:00Z"
    },
    ...
  ],
  "total": 87,
  "limit": 10,
  "offset": 0
}
```

**Subscribe to DistoDam**:
```http
POST /api/v1/wallet/wallet-abc123/subscribe
Content-Type: application/json

{
  "distodam_id": "distodam-001",
  "subscription_fee": 0.0
}

Response 200:
{
  "wallet_id": "wallet-abc123",
  "distodam_id": "distodam-001",
  "status": "subscribed",
  "subscribed_at": "2025-11-14T12:00:00Z"
}
```

**Interface**:
```go
type APIServer interface {
    Start(ctx context.Context) error
    Stop(ctx context.Context) error
    RegisterRoutes(router *mux.Router)
}

type WalletHandler struct {
    balanceManager    BalanceManager
    transactionLogger TransactionLogger
    walletRepo        WalletRepository
    logger            *zap.Logger
    metrics           *MetricsCollector
}
```

**Tests** (28 tests):
- ✅ Create wallet endpoint (4 tests)
- ✅ Get wallet details endpoint (3 tests)
- ✅ Get balance endpoint (3 tests)
- ✅ Get transactions endpoint (pagination) (5 tests)
- ✅ Subscribe to DistoDam endpoint (4 tests)
- ✅ Get UBD status endpoint (3 tests)
- ✅ Invalid wallet ID handling (3 tests)
- ✅ JSON parsing errors (3 tests)

---

### **6. WalletRepository**
**Responsibility**: Database operations for wallet entities

**Interface**:
```go
type WalletRepository interface {
    // Wallet CRUD
    CreateWallet(ctx context.Context, wallet *Wallet) error
    GetWallet(ctx context.Context, walletID string) (*Wallet, error)
    UpdateWallet(ctx context.Context, wallet *Wallet) error
    DeleteWallet(ctx context.Context, walletID string) error
    
    // Wallet queries
    ListWallets(ctx context.Context, limit, offset int) ([]*Wallet, error)
    GetWalletByOwner(ctx context.Context, ownerID string) (*Wallet, error)
    
    // UBD subscription
    UpdateUBDSubscription(ctx context.Context, walletID, distoDamID string) error
    GetUBDSubscription(ctx context.Context, walletID string) (*UBDSubscription, error)
}

type Wallet struct {
    ID              string    `db:"id" json:"id"`
    OwnerID         string    `db:"owner_id" json:"owner_id"`          // Future: link to identity
    BalanceMicroRT  int64     `db:"balance_micro_rt" json:"-"`         // Stored as micro-RT
    Balance         float64   `json:"balance"`                         // Computed: micro_rt / 1M
    Status          string    `db:"status" json:"status"`              // "pending", "active", "suspended"
    CreatedAt       time.Time `db:"created_at" json:"created_at"`
    UpdatedAt       time.Time `db:"updated_at" json:"updated_at"`
    Metadata        string    `db:"metadata" json:"metadata"`          // JSON blob
}

type UBDSubscription struct {
    WalletID      string    `db:"wallet_id" json:"wallet_id"`
    DistoDamID    string    `db:"distodam_id" json:"distodam_id"`
    Status        string    `db:"status" json:"status"`                // "subscribed", "unsubscribed"
    SubscribedAt  time.Time `db:"subscribed_at" json:"subscribed_at"`
    UnsubscribedAt *time.Time `db:"unsubscribed_at" json:"unsubscribed_at,omitempty"`
}
```

**Tests** (20 tests):
- ✅ Create wallet (3 tests)
- ✅ Get wallet by ID (3 tests)
- ✅ Update wallet (3 tests)
- ✅ Delete wallet (2 tests)
- ✅ List wallets with pagination (3 tests)
- ✅ UBD subscription CRUD (4 tests)
- ✅ Database constraints (2 tests)

---

### **7. MetricsCollector**
**Responsibility**: Expose Prometheus metrics for monitoring

**Metrics**:
```go
// UBD metrics
ubd_events_received_total          // Counter: Total UBD events received
ubd_amount_rt_total                // Counter: Total RT received via UBD
ubd_processing_duration_seconds    // Histogram: UBD event processing time

// Wallet metrics
wallet_balance_rt                  // Gauge: Current balance per wallet (labeled by wallet_id)
wallets_created_total              // Counter: Total wallets created
wallets_active_total               // Gauge: Number of active wallets

// Transaction metrics
transactions_logged_total          // Counter: Total transactions logged (labeled by type)
transaction_logging_duration_seconds // Histogram: Transaction logging time

// API metrics
http_requests_total                // Counter: HTTP requests (labeled by endpoint, method, status)
http_request_duration_seconds      // Histogram: Request duration

// Investment metrics (future)
investments_total                  // Counter: Total investments made
investment_amount_rt_total         // Counter: Total RT invested
```

**Tests** (12 tests):
- ✅ Metrics registration (3 tests)
- ✅ Counter increments (3 tests)
- ✅ Gauge updates (3 tests)
- ✅ Histogram observations (3 tests)

---

## 🗄️ **Database Schema**

### **Technology**: PostgreSQL 15+

### **Schema**:

```sql
-- Wallets table
CREATE TABLE wallets (
    id              VARCHAR(64) PRIMARY KEY,        -- "wallet-{uuid}"
    owner_id        VARCHAR(64),                    -- Future: link to identity system
    balance_micro_rt BIGINT NOT NULL DEFAULT 0,     -- Balance in micro-RT (1 RT = 1M µRT)
    status          VARCHAR(20) NOT NULL DEFAULT 'active', -- "pending", "active", "suspended"
    metadata        JSONB,                          -- Extensible metadata
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    
    CONSTRAINT balance_non_negative CHECK (balance_micro_rt >= 0),
    CONSTRAINT valid_status CHECK (status IN ('pending', 'active', 'suspended'))
);

CREATE INDEX idx_wallets_owner_id ON wallets(owner_id);
CREATE INDEX idx_wallets_status ON wallets(status);
CREATE INDEX idx_wallets_created_at ON wallets(created_at DESC);

-- Transactions table
CREATE TABLE transactions (
    id              VARCHAR(64) PRIMARY KEY,        -- "tx-{uuid}"
    wallet_id       VARCHAR(64) NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    type            VARCHAR(30) NOT NULL,           -- "ubd_inflow", "investment", "subscription", etc.
    amount_micro_rt BIGINT NOT NULL,                -- Amount in micro-RT (positive or negative)
    balance_after_micro_rt BIGINT NOT NULL,         -- Wallet balance after transaction
    related_to      VARCHAR(64),                    -- RequestID, ContractID, etc.
    metadata        JSONB,                          -- Extensible metadata
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    
    CONSTRAINT valid_type CHECK (type IN (
        'ubd_inflow', 
        'investment', 
        'investment_return',
        'subscription',
        'transfer_in',
        'transfer_out',
        'activation'
    ))
);

CREATE INDEX idx_transactions_wallet_id ON transactions(wallet_id);
CREATE INDEX idx_transactions_type ON transactions(type);
CREATE INDEX idx_transactions_created_at ON transactions(created_at DESC);
CREATE INDEX idx_transactions_related_to ON transactions(related_to);

-- UBD subscriptions table
CREATE TABLE ubd_subscriptions (
    wallet_id       VARCHAR(64) PRIMARY KEY REFERENCES wallets(id) ON DELETE CASCADE,
    distodam_id     VARCHAR(64) NOT NULL,           -- "distodam-001"
    status          VARCHAR(20) NOT NULL DEFAULT 'subscribed',
    subscribed_at   TIMESTAMP NOT NULL DEFAULT NOW(),
    unsubscribed_at TIMESTAMP,
    
    CONSTRAINT valid_status CHECK (status IN ('subscribed', 'unsubscribed'))
);

CREATE INDEX idx_ubd_subscriptions_distodam_id ON ubd_subscriptions(distodam_id);
CREATE INDEX idx_ubd_subscriptions_status ON ubd_subscriptions(status);

-- Idempotency table (prevent duplicate UBD events)
CREATE TABLE processed_ubd_events (
    request_id      VARCHAR(64) PRIMARY KEY,        -- From UBDFundedEvent
    wallet_id       VARCHAR(64) NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    amount_micro_rt BIGINT NOT NULL,
    processed_at    TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_processed_ubd_events_wallet_id ON processed_ubd_events(wallet_id);
CREATE INDEX idx_processed_ubd_events_processed_at ON processed_ubd_events(processed_at DESC);

-- Future: Investments table (Phase 2 - BidNet integration)
-- CREATE TABLE investments (
--     id              VARCHAR(64) PRIMARY KEY,
--     wallet_id       VARCHAR(64) NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
--     contract_id     VARCHAR(64) NOT NULL,
--     amount_micro_rt BIGINT NOT NULL,
--     status          VARCHAR(20) NOT NULL DEFAULT 'active',
--     invested_at     TIMESTAMP NOT NULL DEFAULT NOW(),
--     returned_at     TIMESTAMP,
--     return_amount_micro_rt BIGINT
-- );
```

**Migration Strategy**:
- Use `golang-migrate/migrate` for schema migrations
- Versioned SQL files: `000001_create_wallets.up.sql`, etc.
- Rollback support: `*.down.sql` files

---

## 📡 **NATS Topics & Message Schemas**

### **Subscribed Topics** (Input):

#### **1. `ubd.funded`** (from DistoDam)
**Purpose**: Receive UBD funding events to credit wallet balances

**Message Schema**:
```go
type UBDFundedEvent struct {
    RequestID   string    `json:"request_id"`    // Idempotency key
    WalletID    string    `json:"wallet_id"`     // Target wallet
    Amount      float64   `json:"amount"`        // RT funded
    FundedAt    time.Time `json:"funded_at"`     // UTC timestamp
    FundedBy    string    `json:"funded_by"`     // "distodam-001"
    TxHash      string    `json:"tx_hash,omitempty"` // Future: blockchain tx
}
```

**Example**:
```json
{
  "request_id": "ubd-req-456",
  "wallet_id": "wallet-abc123",
  "amount": 2.5,
  "funded_at": "2025-11-14T12:30:00Z",
  "funded_by": "distodam-001"
}
```

---

### **Published Topics** (Output - Future):

#### **2. `wallet.investments`** (to BidNet - Phase 2)
**Purpose**: Notify BidNet of new contract investments

**Message Schema**:
```go
type WalletInvestment struct {
    InvestmentID  string    `json:"investment_id"`  // "inv-{uuid}"
    WalletID      string    `json:"wallet_id"`      // Investor wallet
    ContractID    string    `json:"contract_id"`    // BidNet contract
    Amount        float64   `json:"amount"`         // RT invested
    InvestedAt    time.Time `json:"invested_at"`    // UTC timestamp
    ExpectedROI   float64   `json:"expected_roi,omitempty"` // From contract terms
}
```

**Example**:
```json
{
  "investment_id": "inv-789",
  "wallet_id": "wallet-abc123",
  "contract_id": "contract-xyz",
  "amount": 10.0,
  "invested_at": "2025-11-14T13:00:00Z",
  "expected_roi": 0.15
}
```

---

## 🧪 **Testing Strategy**

### **Test Coverage Target**: 95%+

### **Unit Tests** (120+ tests):
- ✅ **Config**: 29 tests (validation, defaults, env parsing)
- ✅ **UBDReceiver**: 18 tests (message processing, error handling, idempotency)
- ✅ **BalanceManager**: 22 tests (credit/debit, transfers, atomicity)
- ✅ **TransactionLogger**: 15 tests (logging, queries, pagination)
- ✅ **APIServer**: 28 tests (endpoints, validation, error responses)
- ✅ **WalletRepository**: 20 tests (CRUD, queries, constraints)
- ✅ **MetricsCollector**: 12 tests (metric registration, updates)

### **Integration Tests** (Embedded NATS + PostgreSQL):
```go
func TestWalletIntegration_UBDFunding(t *testing.T) {
    // Setup: Start embedded NATS + PostgreSQL test container
    natsServer := natstest.RunServer(t)
    defer natsServer.Shutdown()
    
    db := testdb.SetupPostgres(t)
    defer db.Close()
    
    // Create Wallet service components
    config := &Config{...}
    walletRepo := NewWalletRepository(db)
    balanceManager := NewBalanceManager(walletRepo)
    ubdReceiver := NewUBDReceiver(natsServer.ClientURL(), balanceManager)
    
    // Create test wallet
    wallet := &Wallet{ID: "wallet-test", BalanceMicroRT: 0}
    walletRepo.CreateWallet(context.Background(), wallet)
    
    // Publish UBD event to NATS
    natsClient := nats.Connect(natsServer.ClientURL())
    event := UBDFundedEvent{
        RequestID: "ubd-req-123",
        WalletID:  "wallet-test",
        Amount:    5.0,
        FundedAt:  time.Now(),
        FundedBy:  "distodam-001",
    }
    natsClient.PublishJSON("ubd.funded", event)
    
    // Wait for processing
    time.Sleep(100 * time.Millisecond)
    
    // Verify wallet balance updated
    balance, _ := balanceManager.GetBalance(context.Background(), "wallet-test")
    assert.Equal(t, 5.0, balance)
    
    // Verify transaction logged
    txs, _ := walletRepo.GetTransactionsByWallet(context.Background(), "wallet-test", 10, 0)
    assert.Len(t, txs, 1)
    assert.Equal(t, "ubd_inflow", txs[0].Type)
}
```

**Integration Test Scenarios**:
1. ✅ UBD funding flow (DistoDam → Wallet)
2. ✅ Wallet creation via API → database persistence
3. ✅ Balance updates → transaction logging
4. ✅ Concurrent UBD events (race conditions)
5. ✅ Database transaction rollback on errors
6. ✅ NATS reconnection handling

### **E2E Test Script** (PowerShell):
```powershell
# test-wallet-e2e.ps1
# End-to-end test for Wallet service

Write-Host "=== Wallet E2E Test ===" -ForegroundColor Green

# 1. Start services
Write-Host "Starting services..." -ForegroundColor Cyan
docker-compose up -d nats postgres wallet
Start-Sleep -Seconds 5

# 2. Health check
Write-Host "Checking Wallet health..." -ForegroundColor Cyan
$health = Invoke-RestMethod -Uri "http://localhost:3001/health"
if ($health.status -ne "ok") {
    Write-Host "❌ Wallet not healthy" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Wallet healthy" -ForegroundColor Green

# 3. Create wallet
Write-Host "Creating wallet..." -ForegroundColor Cyan
$createResponse = Invoke-RestMethod -Uri "http://localhost:3001/api/v1/wallet" `
    -Method POST -ContentType "application/json" `
    -Body '{"owner_id":"test-user-1"}'
$walletID = $createResponse.id
Write-Host "✅ Wallet created: $walletID" -ForegroundColor Green

# 4. Check initial balance
Write-Host "Checking initial balance..." -ForegroundColor Cyan
$balanceResponse = Invoke-RestMethod -Uri "http://localhost:3001/api/v1/wallet/$walletID/balance"
if ($balanceResponse.balance -ne 0.0) {
    Write-Host "❌ Initial balance incorrect: $($balanceResponse.balance)" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Initial balance: 0.0 RT" -ForegroundColor Green

# 5. Simulate UBD funding (publish to NATS)
Write-Host "Simulating UBD funding..." -ForegroundColor Cyan
docker exec -i robotorq-nats nats pub ubd.funded "{\"request_id\":\"ubd-test-1\",\"wallet_id\":\"$walletID\",\"amount\":10.0,\"funded_at\":\"$(Get-Date -Format o)\",\"funded_by\":\"distodam-test\"}"
Start-Sleep -Seconds 2

# 6. Check updated balance
Write-Host "Checking updated balance..." -ForegroundColor Cyan
$balanceResponse = Invoke-RestMethod -Uri "http://localhost:3001/api/v1/wallet/$walletID/balance"
if ($balanceResponse.balance -ne 10.0) {
    Write-Host "❌ Balance not updated: $($balanceResponse.balance)" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Balance updated: 10.0 RT" -ForegroundColor Green

# 7. Check transaction history
Write-Host "Checking transaction history..." -ForegroundColor Cyan
$txResponse = Invoke-RestMethod -Uri "http://localhost:3001/api/v1/wallet/$walletID/transactions"
if ($txResponse.transactions.Count -ne 1) {
    Write-Host "❌ Transaction count incorrect: $($txResponse.transactions.Count)" -ForegroundColor Red
    exit 1
}
if ($txResponse.transactions[0].type -ne "ubd_inflow") {
    Write-Host "❌ Transaction type incorrect: $($txResponse.transactions[0].type)" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Transaction logged correctly" -ForegroundColor Green

# 8. Subscribe to DistoDam
Write-Host "Subscribing to DistoDam..." -ForegroundColor Cyan
$subscribeResponse = Invoke-RestMethod -Uri "http://localhost:3001/api/v1/wallet/$walletID/subscribe" `
    -Method POST -ContentType "application/json" `
    -Body '{"distodam_id":"distodam-001","subscription_fee":0.0}'
Write-Host "✅ Subscribed to DistoDam" -ForegroundColor Green

Write-Host "`n=== All E2E tests passed! ===" -ForegroundColor Green
```

---

## 🐳 **Docker Deployment**

### **Dockerfile** (Multi-stage build):
```dockerfile
# Stage 1: Build stage
FROM golang:1.24-alpine AS builder

WORKDIR /app

# Install dependencies
RUN apk add --no-cache git

# Copy go.mod and go.sum
COPY go.mod go.sum ./
RUN go mod download

# Copy source code
COPY . .

# Build binary
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o wallet ./cmd/wallet

# Stage 2: Runtime stage
FROM alpine:latest

RUN apk --no-cache add ca-certificates

WORKDIR /root/

# Copy binary from builder
COPY --from=builder /app/wallet .

# Expose HTTP and metrics ports
EXPOSE 3001 9094

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3001/health || exit 1

CMD ["./wallet"]
```

### **docker-compose.yaml** (Integration):
```yaml
version: '3.8'

services:
  wallet:
    build:
      context: ./src/wallet
      dockerfile: Dockerfile
    container_name: robotorq-wallet
    ports:
      - "3001:3001"  # HTTP API
      - "9094:9094"  # Prometheus metrics
    environment:
      HTTP_PORT: "3001"
      NATS_URL: "nats://nats:4222"
      DB_HOST: "postgres"
      DB_PORT: "5432"
      DB_NAME: "wallet_db"
      DB_USER: "wallet"
      DB_PASSWORD: "wallet_pass"
      DB_SSL_MODE: "disable"
      LOG_LEVEL: "info"
      LOG_FORMAT: "json"
      METRICS_ENABLED: "true"
      METRICS_PORT: "9094"
    depends_on:
      - nats
      - postgres
    networks:
      - robotorq
    restart: unless-stopped

  postgres:
    image: postgres:15-alpine
    container_name: robotorq-postgres
    environment:
      POSTGRES_USER: wallet
      POSTGRES_PASSWORD: wallet_pass
      POSTGRES_DB: wallet_db
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./src/wallet/migrations:/docker-entrypoint-initdb.d  # Auto-run migrations
    ports:
      - "5432:5432"
    networks:
      - robotorq
    restart: unless-stopped

volumes:
  postgres_data:

networks:
  robotorq:
    external: true
```

---

## 📝 **Implementation Checklist**

### **Phase 1: Core Infrastructure** (Day 1-2)
- [ ] **Project scaffolding**
  - [ ] Create `src/wallet/` directory structure
  - [ ] Initialize Go module: `go mod init b2b/wallet`
  - [ ] Create `cmd/wallet/main.go` entry point
  - [ ] Create `internal/` packages: config, models, repository, nats, api
  
- [ ] **Config component**
  - [ ] Implement `LoadConfig()` with environment variable parsing
  - [ ] Implement `Validate()` with comprehensive checks
  - [ ] Write 29 unit tests (all passing)
  - [ ] Document all environment variables
  
- [ ] **Database setup**
  - [ ] Create PostgreSQL schema migrations
  - [ ] Implement `WalletRepository` interface
  - [ ] Implement database connection pool
  - [ ] Write 20 repository unit tests
  - [ ] Test migrations (up/down)

- [ ] **Docker setup**
  - [ ] Create Dockerfile (multi-stage build)
  - [ ] Update docker-compose.yaml (add wallet + postgres)
  - [ ] Health check endpoint
  - [ ] Test local build: `docker-compose build wallet`

### **Phase 2: Core Components** (Day 2-3)
- [ ] **BalanceManager**
  - [ ] Implement Credit/Debit operations
  - [ ] Implement atomic Transfer
  - [ ] Implement micro-RT precision handling
  - [ ] Write 22 unit tests (atomicity, concurrency)
  
- [ ] **TransactionLogger**
  - [ ] Implement LogTransaction
  - [ ] Implement query methods (pagination)
  - [ ] Write 15 unit tests
  
- [ ] **UBDReceiver**
  - [ ] Subscribe to `ubd.funded` topic
  - [ ] Implement message parsing & validation
  - [ ] Implement idempotency (check `request_id`)
  - [ ] Integrate with BalanceManager + TransactionLogger
  - [ ] Write 18 unit tests (including error handling)

### **Phase 3: HTTP API** (Day 3-4)
- [ ] **APIServer**
  - [ ] Implement wallet CRUD endpoints
  - [ ] Implement balance endpoint
  - [ ] Implement transactions endpoint (pagination)
  - [ ] Implement DistoDam subscription endpoint
  - [ ] Implement health endpoint
  - [ ] Write 28 API unit tests
  
- [ ] **Middleware**
  - [ ] Request logging (zap)
  - [ ] Error handling (standardized responses)
  - [ ] CORS (if needed for web UI)
  - [ ] Rate limiting (future)

### **Phase 4: Metrics & Monitoring** (Day 4)
- [ ] **MetricsCollector**
  - [ ] Implement Prometheus metrics (counters, gauges, histograms)
  - [ ] Expose `/metrics` endpoint
  - [ ] Write 12 metrics unit tests
  
- [ ] **Observability**
  - [ ] Structured logging (zap) throughout
  - [ ] Add trace IDs to requests
  - [ ] Add Grafana dashboard (optional)

### **Phase 5: Integration & Testing** (Day 5)
- [ ] **Integration tests**
  - [ ] Embedded NATS tests (UBD funding flow)
  - [ ] PostgreSQL test container (testcontainers-go)
  - [ ] End-to-end scenarios (6 scenarios)
  
- [ ] **E2E test script**
  - [ ] Create `test-wallet-e2e.ps1`
  - [ ] Test wallet creation → UBD funding → balance update
  - [ ] Test DistoDam subscription
  - [ ] Verify transaction logging
  
- [ ] **CI/CD**
  - [ ] Update GitHub Actions workflow
  - [ ] Run tests on PR
  - [ ] Build Docker image
  - [ ] Deploy to staging (if applicable)

### **Phase 6: Documentation** (Day 5)
- [ ] **Architecture documentation**
  - [ ] Create `WALLET_ARCHITECTURE.md`
  - [ ] Document component interactions
  - [ ] Document database schema
  - [ ] Document NATS topics
  
- [ ] **API documentation**
  - [ ] OpenAPI/Swagger spec (optional)
  - [ ] Endpoint examples (request/response)
  
- [ ] **README**
  - [ ] Update `src/wallet/README.md`
  - [ ] Quick start guide
  - [ ] Development setup
  - [ ] Testing instructions

---

## 🔮 **Future Phases**

### **Phase 2: BidNet Integration** (Post-MVP)
- [ ] Subscribe to `contracts.approved` topic
- [ ] Implement contract browsing API
- [ ] Implement investment flow:
  - [ ] Debit wallet balance
  - [ ] Publish `wallet.investments` to NATS
  - [ ] Track investment in database
- [ ] Implement investment returns flow:
  - [ ] Subscribe to `contracts.completed` (from BidNet)
  - [ ] Credit wallet with returns
  - [ ] Update investment status

### **Phase 3: Physical RoboTorq Activation**
- [ ] NFC/QR code scanning integration
- [ ] Minimum activation threshold validation
- [ ] Physical → digital RT conversion
- [ ] Activation transaction logging

### **Phase 4: P2P Marketplace**
- [ ] Peer-to-peer transfer API
- [ ] Goods/services listing
- [ ] Escrow system (hold funds during transaction)
- [ ] Reputation system

### **Phase 5: Builder Portal**
- [ ] Contract posting API (Wallet → BidNet)
- [ ] Robot/service/supplier listings
- [ ] Contract management dashboard

### **Phase 6: Advanced Features**
- [ ] Proof-of-work verification interface
- [ ] Verification rewards system
- [ ] TorqVault staking integration
- [ ] Multi-signature wallets
- [ ] Blockchain settlement layer

---

## 🎓 **Design Philosophy: Teaching Through Constraint**

### **Why Physical RoboTorq Activation?** (Future)
The requirement to scan physical RoboTorq before digital wallet activation is **pedagogical**:

1. **Value Anchoring**: Forces users to acquire RT in the real world first
   - Trade goods/services for physical RT coins
   - Understand RT represents **actual robotic work**, not fiat
   - Experience the physical-to-digital bridge

2. **Sybil Resistance**: Prevents unlimited fake wallet creation
   - Physical RT has manufacturing cost (NFC tags, metal/plastic)
   - Minimum threshold (e.g., 100 RT) creates just enough economic barrier to make it valuable.
   - Real-world acquisition proves skin-in-the-game

3. **Learning Curve**: Onboarding is intentionally gradual
   - Step 1: Acquire physical RT (learn value)
   - Step 2: Scan & activate wallet (learn digital bridge)
   - Step 3: Receive UBD (learn network participation)
   - Step 4: Invest/trade (learn economy)

4. **Community Formation**: Real-world RT acquisition builds local networks
   - Find RT holders in your area
   - Trade skills/goods for RT
   - Build trust before entering digital economy

### **Progressive Disclosure**: MVP → Full Feature Set
The Wallet starts **simple** (UBD + investment) and grows **complex** (P2P, builder portal, verification) as users gain experience. This mirrors the RoboTorq ethos:

> **"The network teaches you how to participate by constraining your entry, then expanding your capabilities as you learn."**

---

## 📊 **Success Metrics**

### **MVP Success Criteria**:
- ✅ 95%+ test coverage
- ✅ All integration tests passing
- ✅ E2E test script passes
- ✅ HTTP API responds < 100ms (p99)
- ✅ UBD events processed < 50ms (p99)
- ✅ Zero data loss (all transactions logged)
- ✅ Docker deployment successful
- ✅ Health check endpoint functional

### **Performance Targets**:
- **UBD Throughput**: Handle 1,000 UBD events/sec
- **API Latency**: < 100ms for balance queries (p99)
- **Database**: Support 1M+ wallets, 100M+ transactions
- **Concurrent Users**: 10,000 simultaneous API requests

### **Reliability Targets**:
- **Uptime**: 99.9% (< 43 minutes downtime/month)
- **Data Integrity**: Zero balance discrepancies
- **Idempotency**: 100% duplicate event handling
- **Recovery**: < 1 minute service restart

---

## 🤝 **Integration Dependencies**

### **Depends On** (MVP):
1. **DistoDam**: Must publish `ubd.funded` events
   - Schema: `UBDFundedEvent` (defined in DistoDam REFACTOR_TODO.md)
   - Topic: `ubd.funded`
   - Status: ✅ Planned (Phase 1 of DistoDam refactor)

2. **NATS**: Message broker for pub/sub
   - Status: ✅ Operational
   - Version: 2.12.2+

3. **PostgreSQL**: Database for wallet state
   - Status: 📋 Need to add to docker-compose
   - Version: 15+

### **Consumed By** (Future):
1. **BidNet**: Receives `wallet.investments` events (Phase 2)
2. **Trust**: May integrate for contract-based payments (Phase 3)
3. **Refinery**: Token minting to wallets (Phase 4)
4. **Wallet UI**: Consumes HTTP API (separate project)

---

## 📚 **References**

### **Related Documentation**:
- [DistoDam REFACTOR_TODO.md](../distodam/REFACTOR_TODO.md) - UBD funding source
- [BidNet IMPLEMENTATION_TODO.md](../bidnet/IMPLEMENTATION_TODO.md) - Contract investment target
- [Mint ARCHITECTURE.md](../mint/MINT_ARCHITECTURE.md) - Component pattern reference
- [RoboTorq Paper](../../README.md) - Economic model & UBD concept

### **External Resources**:
- [NATS Documentation](https://docs.nats.io/)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [golang-migrate](https://github.com/golang-migrate/migrate) - Database migrations
- [testcontainers-go](https://github.com/testcontainers/testcontainers-go) - Integration testing

---

## ✅ **Pre-Implementation Checklist**

Before starting implementation, verify:

- [ ] DistoDam REFACTOR_TODO.md reviewed (understand `ubd.funded` schema)
- [ ] BidNet IMPLEMENTATION_TODO.md reviewed (understand future integration)
- [ ] Mint service architecture studied (component pattern reference)
- [ ] PostgreSQL experience/familiarity (database design)
- [ ] NATS pub/sub patterns understood (message handling)
- [ ] Go 1.24+ installed (`go version`)
- [ ] Docker installed (`docker --version`)
- [ ] Development environment ready (VS Code + Go extension)

---

## 🎯 **Final Notes**

This Wallet implementation is **deliberately minimal** for MVP while being **architecturally ready** for the full vision. Key design decisions:

1. **Headless First**: Backend API separates concerns (learned from Digger)
2. **Micro-RT Precision**: Avoids floating-point errors in financial calculations
3. **Idempotency**: Prevents duplicate UBD events from corrupting balances
4. **Extensible Schema**: JSONB metadata fields allow future features without schema changes
5. **Future-Proof**: Component interfaces designed for BidNet, P2P, blockchain integration

**Timeline**: 5 days for MVP (120+ tests, full NATS + database integration, Docker deployment)

**Next Steps**: 
1. Review this document with team
2. Validate `UBDFundedEvent` schema with DistoDam refactor
3. Begin Phase 1 implementation (scaffolding + config)

---

**END OF WALLET IMPLEMENTATION TODO**
