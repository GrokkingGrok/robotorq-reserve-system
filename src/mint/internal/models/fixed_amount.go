package models

// FixedAmount provides deterministic fixed-point arithmetic for stake and joule values.
// Represented as integer smallest units with a decimal scale (power-of-ten base).
// This mirrors the Rust Decimal implementation conceptually but is simplified for Go.
// JSON marshal encodes as string to preserve precision and avoid float rounding.

import (
	"encoding/json"
	"errors"
	"fmt"
)

// Scale: number of decimal places. For RoboStake we choose 6 (micro units) as default.
const DefaultScale = 6

type FixedAmount struct {
	value int64
	scale uint8
}

func NewFixedAmount(value int64, scale uint8) (FixedAmount, error) {
	if scale > 18 {
		return FixedAmount{}, errors.New("scale too high")
	}
	return FixedAmount{value: value, scale: scale}, nil
}

// NewStakeFromFloat converts a float64 stake into FixedAmount using DefaultScale.
// Transitional helper for migrating existing float code paths.
func NewStakeFromFloat(f float64) FixedAmount {
	scaled := int64(f*pow10(int(DefaultScale)) + 0.5) // round half up legacy
	return FixedAmount{value: scaled, scale: DefaultScale}
}

func pow10(n int) float64 {
	p := 1.0
	for i := 0; i < n; i++ {
		p *= 10
	}
	return p
}

// ToFloat converts back to float64 (deprecated path, use FixedAmount arithmetic instead).
func (fa FixedAmount) ToFloat() float64 { return float64(fa.value) / pow10(int(fa.scale)) }

func (fa FixedAmount) Add(other FixedAmount) (FixedAmount, error) {
	if fa.scale != other.scale {
		return FixedAmount{}, errors.New("scale mismatch")
	}
	sum := fa.value + other.value
	return FixedAmount{value: sum, scale: fa.scale}, nil
}

func (fa FixedAmount) Value() int64 { return fa.value }
func (fa FixedAmount) Scale() uint8 { return fa.scale }

// String renders canonical decimal form.
func (fa FixedAmount) String() string {
	if fa.scale == 0 {
		return fmt.Sprintf("%d", fa.value)
	}
	divisor := pow10(int(fa.scale))
	integer := fa.value / int64(divisor)
	frac := fa.value % int64(divisor)
	if frac < 0 {
		frac = -frac
	}
	width := int(fa.scale)
	return fmt.Sprintf("%d.%0*d", integer, width, frac)
}

// MarshalJSON encodes as quoted string to avoid downstream float parsing issues.
func (fa FixedAmount) MarshalJSON() ([]byte, error) { return json.Marshal(fa.String()) }

// UnmarshalJSON accepts either string or number.
func (fa *FixedAmount) UnmarshalJSON(data []byte) error {
	var num float64
	if err := json.Unmarshal(data, &num); err == nil {
		*fa = NewStakeFromFloat(num)
		return nil
	}
	var s string
	if err := json.Unmarshal(data, &s); err != nil {
		return err
	}
	var integer int64
	var fracPart string
	if dot := indexRune(s, '.'); dot >= 0 {
		integerStr := s[:dot]
		fracPart = s[dot+1:]
		fmt.Sscanf(integerStr, "%d", &integer)
		var fracInt int64
		fmt.Sscanf(fracPart, "%d", &fracInt)
		scale := len(fracPart)
		value := integer*int64(pow10(scale)) + fracInt
		*fa = FixedAmount{value: value, scale: uint8(scale)}
		return nil
	}
	fmt.Sscanf(s, "%d", &integer)
	*fa = FixedAmount{value: integer, scale: 0}
	return nil
}

// indexRune minimal helper (avoid importing strings)
func indexRune(s string, r rune) int {
	for i, c := range s {
		if c == r {
			return i
		}
	}
	return -1
}
