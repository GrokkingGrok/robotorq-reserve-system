module b2b/trust

go 1.24

require github.com/nats-io/nats.go v1.47.0

require (
	b2b/natsx v0.0.0
	github.com/beorn7/perks v1.0.1 // indirect
	github.com/cespare/xxhash/v2 v2.3.0 // indirect
	github.com/munnerz/goautoneg v0.0.0-20191010083416-a7dc8b61c822 // indirect
	github.com/prometheus/client_model v0.6.2 // indirect
	github.com/prometheus/common v0.66.1 // indirect
	github.com/prometheus/procfs v0.16.1 // indirect
	go.uber.org/multierr v1.10.0 // indirect
	go.yaml.in/yaml/v2 v2.4.2 // indirect
	google.golang.org/protobuf v1.36.8 // indirect
)

require (
	github.com/klauspost/compress v1.18.0 // indirect
	github.com/nats-io/nkeys v0.4.11 // indirect
	github.com/nats-io/nuid v1.0.1 // indirect
	github.com/prometheus/client_golang v1.23.2
	go.uber.org/zap v1.27.0
	golang.org/x/crypto v0.37.0 // indirect
	golang.org/x/sys v0.35.0 // indirect
)

replace b2b/natsx => ../natsx
