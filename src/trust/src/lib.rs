pub mod config;
pub mod handlers;
pub mod http;
pub mod metrics;
pub mod models;
pub mod store;

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // smoke test
        assert_eq!(2 + 2, 4);
    }
}
