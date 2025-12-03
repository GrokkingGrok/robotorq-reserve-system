use std::collections::HashMap;

#[test]
fn propagation_helpers_roundtrip_integration() {
    // This integration-style test uses the public propagation helpers to
    // exercise the end-to-end insertion/extraction flow in a higher-level
    // test harness (outside of the propagation module unit tests).
    let mut headers = HashMap::new();

    // Use the same helper the library exposes (import path is the public API).
    commons::util::tracing::inject_trace_context(
        &mut headers,
        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        Some("vendor=integration"),
    );

    let extracted = commons::util::tracing::extract_trace_context(&headers)
        .expect("traceparent should be present after injection");

    assert_eq!(extracted.0, "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01");
    assert_eq!(extracted.1.as_deref(), Some("vendor=integration"));
}
