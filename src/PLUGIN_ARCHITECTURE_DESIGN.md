# Plugin Architecture Design for Trust, BidNet, Wallet

**Date**: November 21, 2025  
**Status**: Design reference for Phase 10+  
**Purpose**: Blueprint for extensible service design without over-engineering

---

## 🎯 Core Principle

**"Bare bones implementation with plugin interfaces, not plugins themselves."**

- Phase 10: Build the **skeleton** (builtins only, config-driven)
- Phase 11+: Community adds **plugins** as interfaces
- No premature optimization, no unused abstractions

---

## 1. TRUST SERVICE - Plugin Architecture

### 1.1 Plugin Interfaces

```go
// trust/internal/plugins/interfaces.go

package plugins

import (
    "context"
    "errors"
    "robotorq/models"
)

// Appraiser evaluates opportunities and creates contracts
type Appraiser interface {
    // EvaluateOpportunity returns a Contract if opportunity meets criteria, nil otherwise
    EvaluateOpportunity(ctx context.Context, opp *models.Opportunity) (*models.Contract, error)
    
    // Name returns the appraiser's unique name (for logging, metrics, config)
    Name() string
}

// Executor runs a contract (calls Digger API, hardware, etc.)
type Executor interface {
    // Execute runs the contract to completion
    Execute(ctx context.Context, contract *models.Contract) error
    
    // CanExecute checks if executor can handle this contract (type check)
    CanExecute(ctx context.Context, contract *models.Contract) bool
    
    // Name returns the executor's unique name
    Name() string
}

// Validator pre-checks a contract before execution
// (credit checks, reputation, collateral, etc.)
type Validator interface {
    // Validate returns true if contract passes validation
    Validate(ctx context.Context, contract *models.Contract) (bool, error)
    
    // Name returns the validator's unique name
    Name() string
}

// Processor runs after successful execution (bookkeeping, analytics, etc.)
type Processor interface {
    // Process runs post-execution logic
    Process(ctx context.Context, executed *models.Contract) error
    
    // Name returns the processor's unique name
    Name() string
}

// EventHook receives lifecycle events (webhooks, logging, analytics, etc.)
type EventHook interface {
    // OnContractCreated fires when contract is created
    OnContractCreated(ctx context.Context, contract *models.Contract) error
    
    // OnContractValidated fires when contract passes validation
    OnContractValidated(ctx context.Context, contract *models.Contract) error
    
    // OnContractFunded fires when DistoDam funds contract
    OnContractFunded(ctx context.Context, contract *models.Contract) error
    
    // OnContractExecuting fires when execution starts
    OnContractExecuting(ctx context.Context, contract *models.Contract) error
    
    // OnContractExecuted fires when execution completes
    OnContractExecuted(ctx context.Context, contract *models.Contract, result error) error
    
    // Name returns the hook's unique name
    Name() string
}
```

### 1.2 Plugin Registry

```go
// trust/internal/plugins/registry.go

package plugins

import (
    "fmt"
    "sync"
)

type PluginRegistry[T interface{ Name() string }] struct {
    mu      sync.RWMutex
    plugins map[string]T
}

func NewRegistry[T interface{ Name() string }]() *PluginRegistry[T] {
    return &PluginRegistry[T]{
        plugins: make(map[string]T),
    }
}

func (r *PluginRegistry[T]) Register(name string, plugin T) error {
    r.mu.Lock()
    defer r.mu.Unlock()
    
    if _, exists := r.plugins[name]; exists {
        return fmt.Errorf("plugin already registered: %s", name)
    }
    
    r.plugins[name] = plugin
    return nil
}

func (r *PluginRegistry[T]) Get(name string) (T, error) {
    r.mu.RLock()
    defer r.mu.RUnlock()
    
    plugin, ok := r.plugins[name]
    if !ok {
        var zero T
        return zero, fmt.Errorf("plugin not found: %s", name)
    }
    
    return plugin, nil
}

func (r *PluginRegistry[T]) List() []string {
    r.mu.RLock()
    defer r.mu.RUnlock()
    
    names := make([]string, 0, len(r.plugins))
    for name := range r.plugins {
        names = append(names, name)
    }
    return names
}
```

### 1.3 Trust Service with Plugins

```go
// trust/internal/service.go

package trust

import (
    "context"
    "robotorq/models"
    "robotorq/trust/internal/plugins"
)

type TrustService struct {
    appraiserRegistry  *plugins.PluginRegistry[plugins.Appraiser]
    executorRegistry   *plugins.PluginRegistry[plugins.Executor]
    validatorRegistry  *plugins.PluginRegistry[plugins.Validator]
    processorRegistry  *plugins.PluginRegistry[plugins.Processor]
    hookRegistry       *plugins.PluginRegistry[plugins.EventHook]
    
    config Config
}

type Config struct {
    // Which appraiser to use
    ActiveAppraiser string
    
    // Which executor to use
    ActiveExecutor string
    
    // Which validators to run (all must pass)
    ActiveValidators []string
    
    // Which processors to run (all run, errors logged)
    ActiveProcessors []string
    
    // Which hooks to fire (all fire, errors logged)
    EventHooks []string
}

func NewTrustService(cfg Config) *TrustService {
    return &TrustService{
        appraiserRegistry:  plugins.NewRegistry[plugins.Appraiser](),
        executorRegistry:   plugins.NewRegistry[plugins.Executor](),
        validatorRegistry:  plugins.NewRegistry[plugins.Validator](),
        processorRegistry:  plugins.NewRegistry[plugins.Processor](),
        hookRegistry:       plugins.NewRegistry[plugins.EventHook](),
        config:             cfg,
    }
}

// AppraioseOpportunity evaluates using active appraiser
func (t *TrustService) AppraioseOpportunity(ctx context.Context, opp *models.Opportunity) (*models.Contract, error) {
    appraiser, err := t.appraiserRegistry.Get(t.config.ActiveAppraiser)
    if err != nil {
        return nil, err
    }
    
    contract, err := appraiser.EvaluateOpportunity(ctx, opp)
    if err != nil {
        return nil, err
    }
    
    if contract != nil {
        // Fire event hooks
        for _, hookName := range t.config.EventHooks {
            if hook, err := t.hookRegistry.Get(hookName); err == nil {
                hook.OnContractCreated(ctx, contract)  // Ignore errors
            }
        }
    }
    
    return contract, nil
}

// ExecuteContract runs through full validation + execution pipeline
func (t *TrustService) ExecuteContract(ctx context.Context, contract *models.Contract) error {
    // 1. Fire validation hooks
    for _, hookName := range t.config.EventHooks {
        if hook, err := t.hookRegistry.Get(hookName); err == nil {
            hook.OnContractValidated(ctx, contract)
        }
    }
    
    // 2. Run all validators (must all pass)
    for _, validatorName := range t.config.ActiveValidators {
        validator, err := t.validatorRegistry.Get(validatorName)
        if err != nil {
            return err
        }
        
        ok, err := validator.Validate(ctx, contract)
        if err != nil {
            return err
        }
        if !ok {
            return fmt.Errorf("validation failed: %s", validatorName)
        }
    }
    
    // 3. Fire execution start hooks
    for _, hookName := range t.config.EventHooks {
        if hook, err := t.hookRegistry.Get(hookName); err == nil {
            hook.OnContractExecuting(ctx, contract)
        }
    }
    
    // 4. Execute using active executor
    executor, err := t.executorRegistry.Get(t.config.ActiveExecutor)
    if err != nil {
        return err
    }
    
    execErr := executor.Execute(ctx, contract)
    
    // 5. Fire execution complete hooks (regardless of error)
    for _, hookName := range t.config.EventHooks {
        if hook, err := t.hookRegistry.Get(hookName); err == nil {
            hook.OnContractExecuted(ctx, contract, execErr)
        }
    }
    
    if execErr != nil {
        return execErr
    }
    
    // 6. Run all processors (errors logged, not returned)
    for _, processorName := range t.config.ActiveProcessors {
        processor, err := t.processorRegistry.Get(processorName)
        if err != nil {
            slog.Error("processor not found", "name", processorName)
            continue
        }
        
        if err := processor.Process(ctx, contract); err != nil {
            slog.Error("processor failed", "name", processorName, "error", err)
        }
    }
    
    return nil
}

// RegisterPlugin allows dynamic plugin registration
func (t *TrustService) RegisterAppraiser(name string, a plugins.Appraiser) error {
    return t.appraiserRegistry.Register(name, a)
}

func (t *TrustService) RegisterExecutor(name string, e plugins.Executor) error {
    return t.executorRegistry.Register(name, e)
}

func (t *TrustService) RegisterValidator(name string, v plugins.Validator) error {
    return t.validatorRegistry.Register(name, v)
}

func (t *TrustService) RegisterProcessor(name string, p plugins.Processor) error {
    return t.processorRegistry.Register(name, p)
}

func (t *TrustService) RegisterEventHook(name string, h plugins.EventHook) error {
    return t.hookRegistry.Register(name, h)
}
```

### 1.4 Builtin Implementations (Phase 10)

```go
// trust/internal/builtins/roi_appraiser.go

package builtins

import (
    "context"
    "fmt"
    "robotorq/models"
)

type ROIAppraiser struct {
    threshold float64
}

func NewROIAppraiser(threshold float64) *ROIAppraiser {
    return &ROIAppraiser{threshold: threshold}
}

func (a *ROIAppraiser) EvaluateOpportunity(ctx context.Context, opp *models.Opportunity) (*models.Contract, error) {
    if opp.ROI < a.threshold {
        return nil, nil  // Rejected, no contract
    }
    
    return &models.Contract{
        ID:          generateContractID(),
        OpportunityID: opp.ID,
        RequiredRT: opp.RequiredRT,
        ROI:        opp.ROI,
        Status:     "created",
        CreatedAt:  time.Now(),
    }, nil
}

func (a *ROIAppraiser) Name() string {
    return "roi_appraiser"
}
```

```go
// trust/internal/builtins/digger_executor.go

package builtins

import (
    "context"
    "fmt"
    "net/http"
    "robotorq/models"
)

type DiggerExecutor struct {
    httpClient *http.Client
    diggerURL  string
}

func NewDiggerExecutor(diggerURL string) *DiggerExecutor {
    return &DiggerExecutor{
        httpClient: &http.Client{Timeout: 30 * time.Second},
        diggerURL:  diggerURL,
    }
}

func (e *DiggerExecutor) Execute(ctx context.Context, contract *models.Contract) error {
    // Call Digger API to execute contract
    resp, err := e.httpClient.Post(
        fmt.Sprintf("%s/contract/execute", e.diggerURL),
        "application/json",
        // ... request body
    )
    // ... handle response
    return err
}

func (e *DiggerExecutor) CanExecute(ctx context.Context, contract *models.Contract) bool {
    return true  // Can execute any contract
}

func (e *DiggerExecutor) Name() string {
    return "digger_executor"
}
```

### 1.5 Configuration (trust-config.yaml)

```yaml
# trust/config/trust-config.yaml

plugins:
  appraisers:
    roi_appraiser:
      type: builtin
      enabled: true
      config:
        threshold: 0.10
    
    ml_appraiser:
      type: go_plugin
      enabled: false
      path: ./plugins/ml_appraiser.so
      config:
        model_url: "https://..."
        threshold: 0.75

service:
  active_appraiser: roi_appraiser
  active_executor: digger_executor
  active_validators: []
  active_processors: []
  event_hooks: [logging]

logging:
  level: info
  format: json
```

---

## 2. BIDNET SERVICE - Plugin Architecture

### 2.1 BidNet Plugin Interfaces

```go
// bidnet/internal/plugins/interfaces.go

package plugins

import (
    "context"
    "robotorq/models"
)

// RateEvaluator determines market rates (Uniswap, Chainlink, custom API, ML, etc.)
type RateEvaluator interface {
    // GetRate returns RT→USDC rate (or error)
    GetRate(ctx context.Context, pair string) (float64, error)
    
    // Name returns the evaluator's unique name
    Name() string
}

// TradeExecutor executes trades (Uniswap, custom DEX, over-the-counter, etc.)
type TradeExecutor interface {
    // ExecuteTrade swaps RT for USDC (or vice versa)
    ExecuteTrade(ctx context.Context, trade *models.Trade) error
    
    // EstimateSlippage estimates slippage for given amount
    EstimateSlippage(ctx context.Context, amountRT float64) (float64, error)
    
    // Name returns the executor's unique name
    Name() string
}

// TradeValidator validates trades before execution
// (slippage limits, liquidity checks, KYC, etc.)
type TradeValidator interface {
    // ValidateTrade returns true if trade meets criteria
    ValidateTrade(ctx context.Context, trade *models.Trade) (bool, error)
    
    // Name returns the validator's unique name
    Name() string
}

// ReputationChecker evaluates counterparty reputation
type ReputationChecker interface {
    // CheckReputation returns reputation score (0-100)
    CheckReputation(ctx context.Context, walletID string) (int, error)
    
    // Name returns the checker's unique name
    Name() string
}
```

### 2.2 Builtin Implementations (Phase 10)

- **SimpleRateEvaluator**: Returns hardcoded rate (development only)
- **SimpleTradeValidator**: Checks basic slippage thresholds
- **SimpleReputationChecker**: Returns constant reputation score

---

## 3. WALLET SERVICE - Plugin Architecture

### 3.1 Wallet Plugin Interfaces

```go
// wallet/internal/plugins/interfaces.go

package plugins

import (
    "context"
    "crypto/ed25519"
    "robotorq/models"
)

// Signer manages key signing (hardware wallet, cloud KMS, local key, etc.)
type Signer interface {
    // Sign signs a message with the wallet's private key
    Sign(ctx context.Context, message []byte) ([]byte, error)
    
    // PublicKey returns the wallet's public key
    PublicKey() ed25519.PublicKey
    
    // Name returns the signer's unique name
    Name() string
}

// TransactionValidator validates transactions before sending
// (custom rules, spam filters, rate limits, etc.)
type TransactionValidator interface {
    // ValidateTransaction returns true if transaction is valid
    ValidateTransaction(ctx context.Context, tx *models.Transaction) (bool, error)
    
    // Name returns the validator's unique name
    Name() string
}

// EventNotifier sends notifications (Discord, email, Telegram, etc.)
type EventNotifier interface {
    // OnTransactionSent fires when transaction is sent
    OnTransactionSent(ctx context.Context, tx *models.Transaction) error
    
    // OnTransactionReceived fires when transaction is received
    OnTransactionReceived(ctx context.Context, tx *models.Transaction) error
    
    // OnBalanceChanged fires when balance changes
    OnBalanceChanged(ctx context.Context, newBalance float64, oldBalance float64) error
    
    // Name returns the notifier's unique name
    Name() string
}

// BackupProvider manages wallet backups
// (local file, cloud storage, hardware backup, etc.)
type BackupProvider interface {
    // Backup saves wallet state
    Backup(ctx context.Context, state *models.WalletState) error
    
    // Restore loads wallet state
    Restore(ctx context.Context) (*models.WalletState, error)
    
    // Name returns the provider's unique name
    Name() string
}
```

### 3.2 Builtin Implementations (Phase 10)

- **LocalSigner**: Sign with local private key
- **NoopValidator**: Accept all transactions
- **LoggingNotifier**: Log events to stdout

---

## 4. Implementation Roadmap for Phase 10

### For Each Service (Trust, BidNet, Wallet):

#### **Step 1: Define Interfaces** (1-2 hours)
- [ ] Write plugin interface definitions (what can be extended?)
- [ ] Keep minimal (5-10 methods per interface)

#### **Step 2: Build Registry** (1 hour)
- [ ] Generic `PluginRegistry[T]` (reuse across all services)
- [ ] Methods: `Register`, `Get`, `List`

#### **Step 3: Create Builtins** (2-3 hours)
- [ ] Implement 1-2 builtin plugins per interface
- [ ] Keep builtins simple (focus on core logic)

#### **Step 4: Wire Service** (1-2 hours)
- [ ] Update service to use registry + plugins
- [ ] Replace hardcoded logic with plugin calls

#### **Step 5: Config-Driven** (1 hour)
- [ ] Load config from YAML
- [ ] Instantiate builtins based on config
- [ ] Allow `active_plugin` settings

#### **Step 6: Document Plugin API** (1 hour)
- [ ] Write plugin development guide
- [ ] Show example: "How to write your own appraiser"

---

## 5. Example: Writing a Custom Plugin (For Community)

Once Phase 10 is complete, a community member can write:

```go
// github.com/community/trust-ml-appraiser/main.go
package main

import (
    "context"
    "github.com/GrokkingGrok/robotorq/models"
    "github.com/GrokkingGrok/robotorq/trust/plugins"
    "github.com/tensorflow/tensorflow-go"
)

type MLAppraiser struct {
    model *tensorflow.SavedModel
}

func (a *MLAppraiser) EvaluateOpportunity(ctx context.Context, opp *models.Opportunity) (*models.Contract, error) {
    // Run ML model
    score := a.model.Predict(map[string]interface{}{
        "roi": opp.ROI,
        "builder_reputation": opp.BuilderReputation,
        "collateral": opp.Collateral,
    })
    
    if score >= 0.75 {
        return &models.Contract{...}, nil
    }
    return nil, nil  // Rejected
}

func (a *MLAppraiser) Name() string {
    return "ml_appraiser"
}

// Export the plugin
var Plugin = &MLAppraiser{
    model: loadModel(),
}
```

Compile: `go build -buildmode=plugin -o ml_appraiser.so`

Then update config:

```yaml
plugins:
  appraisers:
    ml_appraiser:
      type: go_plugin
      enabled: true
      path: ./plugins/ml_appraiser.so
```

Done. No forking, no rebuilding Trust service, no code changes needed.

---

## 6. Key Design Decisions

| Decision | Rationale | Impact |
|----------|-----------|--------|
| **Generic Registry** | Single registry type, reusable across all plugin types | Simple, DRY, testable |
| **Builtins only in Phase 10** | No plugin loading infrastructure until needed | Fast MVP, proven architecture |
| **Config-driven plugins** | No hardcoding of which plugins run | Easy to swap configurations, A/B test |
| **Error handling varies** | Validators/Executors return errors; Processors/Hooks log errors | Failed validation = stop; Failed post-processing = continue |
| **Hooks fire asynchronously** | Separate goroutines, errors logged not returned | Slow hooks don't block transaction |
| **No hot reload Phase 10** | Plugins loaded at startup only | Simpler, safer, sufficient for MVP |

---

## 7. Testing Strategy

### Unit Tests
```go
// trust/internal/service_test.go

func TestTrustServiceAppraises(t *testing.T) {
    // Mock appraiser
    mockAppraiser := &MockAppraiser{approved: true}
    
    service := trust.NewTrustService(Config{ActiveAppraiser: "mock"})
    service.RegisterAppraiser("mock", mockAppraiser)
    
    contract, _ := service.AppraioseOpportunity(context.Background(), &Opportunity{ROI: 0.15})
    assert.NotNil(t, contract)
}
```

### Integration Tests
```go
func TestFullPipeline(t *testing.T) {
    // Setup all plugins
    service := setupWithAllBuiltins()
    
    // Run full cycle
    opp := &Opportunity{...}
    contract, _ := service.AppraioseOpportunity(context.Background(), opp)
    err := service.ExecuteContract(context.Background(), contract)
    
    assert.NoError(t, err)
}
```

### Plugin Tests
```go
// Plugin authors test their plugins
// Example: github.com/community/trust-ml-appraiser/appraiser_test.go

func TestMLAppraiserHighROI(t *testing.T) {
    appraiser := NewMLAppraiser(loadTestModel())
    
    opp := &Opportunity{ROI: 0.20, ...}
    contract, _ := appraiser.EvaluateOpportunity(context.Background(), opp)
    
    assert.NotNil(t, contract)
}
```

---

## 8. Deployment

### Phase 10 (Builtins Only)
```bash
docker run robotorq/trust:1.0.0 \
  -c trust-config.yaml
```

### Phase 11+ (With Plugins)
```bash
# Mount plugins directory
docker run -v ./plugins:/app/plugins robotorq/trust:1.0.0 \
  -c trust-config.yaml
```

---

## 9. Checklist for Phase 10

- [ ] Define plugin interfaces for Trust
- [ ] Define plugin interfaces for BidNet
- [ ] Define plugin interfaces for Wallet
- [ ] Build generic PluginRegistry[T]
- [ ] Implement 1-2 builtins per service
- [ ] Wire service to use registry
- [ ] Config file support (YAML)
- [ ] Unit tests for registry
- [ ] Integration tests for full pipeline
- [ ] Plugin development guide (README)
- [ ] Example custom plugin (in repo)

---

## 10. Future Extensions (Phase 11+)

- Go plugin loading (`.so` files)
- WASM plugin loading (`.wasm` files)
- HTTP/RPC plugin loading (external services)
- Hot plugin reloading (no restart)
- Plugin dependency resolution
- Plugin versioning & compatibility checks
- Plugin marketplace/registry

---

**This design ensures:**

✅ Core services are **simple and clean** (builtins only)  
✅ Extensibility is **baked in** (interfaces defined upfront)  
✅ Community **knows how to extend** (clear plugin API)  
✅ No over-engineering (only implement what you need now)  
✅ Zero lock-in (write plugins in any language eventually)  

**Build bare bones. Ship interfaces. Let community add plugins.** 🚀
