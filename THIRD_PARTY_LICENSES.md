# Third-Party Dependencies

The RoboTorq Reserve System uses the following open-source libraries. We are grateful to the maintainers and contributors of these projects.

## Go Dependencies (Mint, Refinery, Trust, Wallet, Printer)

### NATS Messaging (Apache 2.0)
- `github.com/nats-io/nats.go` - Go client for NATS messaging system
- `github.com/nats-io/nats-server/v2` - NATS server core
- `github.com/nats-io/jwt/v2`, `nkeys`, `nuid` - NATS authentication and identity

### Prometheus Observability (Apache 2.0)
- `github.com/prometheus/client_golang` - Metrics collection and exposition
- `github.com/prometheus/client_model`, `common`, `procfs` - Prometheus client dependencies

### Post-Quantum Cryptography
- `github.com/open-quantum-safe/liboqs-go` (MIT) - Quantum-resistant cryptographic algorithms (Dilithium5, Kyber)
- `github.com/cloudflare/circl` (BSD-3-Clause) - Cloudflare cryptographic library

### Utilities
- `github.com/google/uuid` (BSD-3-Clause) - UUID generation
- `gopkg.in/yaml.v3` (MIT/Apache 2.0 dual) - YAML parsing
- `github.com/stretchr/testify` (MIT) - Testing framework
- `github.com/klauspost/compress` (Apache 2.0/BSD-3-Clause) - Compression algorithms

### Go Standard Library Extensions (BSD-3-Clause)
- `golang.org/x/crypto`, `net`, `sys`, `sync`, `time` - Extended standard library functionality

## Rust Dependencies (Digger)

*(See `src/digger/Cargo.toml` for complete Rust dependency list)*

- `tokio` (MIT) - Async runtime
- `serde`, `serde_json` (MIT/Apache 2.0) - Serialization
- `sha2` (MIT/Apache 2.0) - SHA-256 hashing
- `chrono` (MIT/Apache 2.0) - Date/time handling

## Python Dependencies (Testing/Scripts)

- `nats-py` (Apache 2.0) - NATS client for Python
- `pytest` (MIT) - Testing framework

---

**Note**: This project (RoboTorq Reserve System) is licensed under MIT License (see `LICENSE`). All third-party dependencies listed above use permissive licenses compatible with MIT licensing.

For full license texts of each dependency, see their respective repositories or run:
- Go: `go list -m -json all` (in any service directory)
- Rust: `cargo metadata --format-version 1` (in `src/digger/`)
- Python: `pip-licenses` (requires `pip install pip-licenses`)
