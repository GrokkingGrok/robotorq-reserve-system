//! Trace context propagation utilities
//!
//! Provides minimal helpers to inject and extract W3C Trace Context
//! (`traceparent` and `tracestate`) into/from a simple key/value map such as
//! HTTP headers or message header maps. The helpers are intentionally small
//! and avoid any heavy dependencies so they can be used in low-level
//! adapters (e.g., NATS producers/consumers) without pulling in tracing
//! crates at that layer.
//!
//! Example (HTTP headers):
//! ```rust
//! use std::collections::HashMap;
//! use commons::util::tracing::inject_trace_context;
//!
//! let mut headers = HashMap::new();
//! inject_trace_context(&mut headers, "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01", None);
//! assert_eq!(headers.get("traceparent").map(|s| s.as_str()), Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"));
//! ```

use std::collections::HashMap;

/// Inject W3C trace context into a generic key/value map.
///
/// * `map` - destination map (e.g., HTTP headers or messaging headers)
/// * `traceparent` - required `traceparent` string in W3C format
/// * `tracestate` - optional `tracestate` value
///
/// This function performs a simple string insertion and does not validate
/// the `traceparent` format beyond storing it. Adapters that need
/// strict validation should apply format checks before calling this helper.
pub fn inject_trace_context(map: &mut HashMap<String, String>, traceparent: &str, tracestate: Option<&str>) {
    map.insert("traceparent".to_string(), traceparent.to_string());
    if let Some(ts) = tracestate {
        map.insert("tracestate".to_string(), ts.to_string());
    }
}

/// Extract W3C trace context from a generic key/value map.
///
/// Returns `Some((traceparent, tracestate_opt))` if `traceparent` is present,
/// otherwise returns `None`.
pub fn extract_trace_context(map: &HashMap<String, String>) -> Option<(String, Option<String>)> {
    match map.get("traceparent") {
        Some(tp) => Some((tp.clone(), map.get("tracestate").cloned())),
        None => None,
    }
}

/// Convenience: convert from iterator of header-like pairs to a HashMap and extract context.
///
/// Useful for adapters that receive headers in a slice or iterator form.
pub fn extract_from_iter<I, K, V>(iter: I) -> Option<(String, Option<String>)>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut map = HashMap::new();
    for (k, v) in iter {
        map.insert(k.as_ref().to_string(), v.as_ref().to_string());
    }
    extract_trace_context(&map)
}
