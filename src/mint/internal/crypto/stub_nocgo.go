//go:build !cgo

package crypto

import "log/slog"

// FalconVerifier stub for non-CGO builds (skips real liboqs verification).
type FalconVerifier struct{}

func NewFalconVerifier() *FalconVerifier { return &FalconVerifier{} }

func (fv *FalconVerifier) VerifyPhase2Ingot(ingotID, branchHash string, hashCount int, timestamp, signatureHex, publicKeyHex string) error {
    // No-op verification in stub mode.
    return nil
}

// SPHINCSPlusSigner stub for non-CGO builds.
type SPHINCSPlusSigner struct {
    publicKey []byte
}

func NewSPHINCSPlusSigner(logger *slog.Logger) (*SPHINCSPlusSigner, error) {
    return &SPHINCSPlusSigner{publicKey: []byte("stub")}, nil
}

func (s *SPHINCSPlusSigner) SignPhase3Unit(unitID, merkleRoot, mintedAt string) (string, string, error) {
    // Return deterministic stub signature/public key hex.
    return "stub-signature", "73747562", nil // "stub" in hex
}

func (s *SPHINCSPlusSigner) VerifyPhase3Unit(unitID, merkleRoot, mintedAt, signatureHex, publicKeyHex string) error {
    return nil
}

func (s *SPHINCSPlusSigner) GetPublicKey() string { return "73747562" }
