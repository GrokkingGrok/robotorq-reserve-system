module b2b/trust

go 1.24

require github.com/nats-io/nats.go v1.47.0

require github.com/stretchr/testify v1.11.1 // indirect

require (
	b2b/natsx v0.0.0
	go.uber.org/multierr v1.10.0 // indirect
)

require (
	github.com/klauspost/compress v1.18.0 // indirect
	github.com/nats-io/nkeys v0.4.11 // indirect
	github.com/nats-io/nuid v1.0.1 // indirect
	go.uber.org/zap v1.27.0
	golang.org/x/crypto v0.37.0 // indirect
	golang.org/x/sys v0.35.0 // indirect
)

replace b2b/natsx => ../natsx
