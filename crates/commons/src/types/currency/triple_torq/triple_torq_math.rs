//! TripleTorq accounting math.
//! These helpers operate on monetary circulation balances (not raw ore measurement).
//! Invariants:
//! - 1 RoboTorq = 1_000 TokenTorq
//! - 1 TokenTorq = 3_600 JouleTorq
//! - tokentorq_balance < 1_000
//! - jouletorq_balance < 3_600
//! The canonical form keeps sub-balances within bounds rolling upward.

pub const TOKEN_TORQS_PER_ROBOTORQ: u128 = 1_000;
pub const JOULE_TORQS_PER_TOKEN_TORQ: u128 = 3_600;
pub const JOULE_TORQS_PER_ROBOTORQ: u128 = TOKEN_TORQS_PER_ROBOTORQ * JOULE_TORQS_PER_TOKEN_TORQ; // 3_600_000

/// Normalize raw (robot, token, joule) into canonical bounded representation.
pub fn normalize(mut robotorq: u128, mut tokentorq: u128, mut jouletorq: u128) -> (u128, u16, u16) {
	// Roll excess JouleTorq into TokenTorq.
	if jouletorq >= JOULE_TORQS_PER_TOKEN_TORQ {
		let extra_tokens = jouletorq / JOULE_TORQS_PER_TOKEN_TORQ;
		tokentorq += extra_tokens;
		jouletorq %= JOULE_TORQS_PER_TOKEN_TORQ;
	}
	// Roll excess TokenTorq into RoboTorq.
	if tokentorq >= TOKEN_TORQS_PER_ROBOTORQ {
		let extra_robotorq = tokentorq / TOKEN_TORQS_PER_ROBOTORQ;
		robotorq += extra_robotorq;
		tokentorq %= TOKEN_TORQS_PER_ROBOTORQ;
	}
	// Safe casts (post-normalization bounds guaranteed).
	(robotorq, tokentorq as u16, jouletorq as u16)
}

/// Convert a canonical triple to total JouleTorq units.
pub fn to_smallest_units(robotorq: u128, tokentorq: u16, jouletorq: u16) -> u128 {
	robotorq * JOULE_TORQS_PER_ROBOTORQ + (tokentorq as u128) * JOULE_TORQS_PER_TOKEN_TORQ + (jouletorq as u128)
}

/// Decompose total JouleTorq units into canonical triple balances.
pub fn from_smallest_units(total: u128) -> (u128, u16, u16) {
	let robotorq = total / JOULE_TORQS_PER_ROBOTORQ;
	let rem_after_robot = total % JOULE_TORQS_PER_ROBOTORQ;
	let tokentorq = rem_after_robot / JOULE_TORQS_PER_TOKEN_TORQ;
	let jouletorq = rem_after_robot % JOULE_TORQS_PER_TOKEN_TORQ;
	(robotorq, tokentorq as u16, jouletorq as u16)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn normalize_rolls_up() {
		let (r,t,j) = normalize(0, 1_500, 7_500); // 1500 tokens, 7500 joules
		// 1500 tokens -> 1 robot + 500 tokens; 7500 joules -> 2 tokens + 300 joules
		// Combined before final token normalization: robot=0+1=1, token=500+2=502, joule=300
		assert_eq!((r,t,j), (1, 502, 300));
	}

	#[test]
	fn round_trip_smallest_units() {
		let (r,t,j) = normalize(2, 1234, 9999);
		let total = to_smallest_units(r, t, j);
		let (r2,t2,j2) = from_smallest_units(total);
		assert_eq!((r,t,j), (r2,t2,j2));
	}
}
