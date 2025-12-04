use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1); // skip binary name

    let backend = args.next().unwrap_or_else(|| "sqlite".to_string());

    match backend.as_str() {
        "sqlite" => {
            let dburl = args
                .next()
                .or_else(|| env::var("DATABASE_URL").ok())
                .unwrap_or_else(|| "sqlite://robotorq.db".to_string());
            println!("Running sqlite migrations against: {}", dburl);
            commons::util::persistence::sqlite::run_migrations_with_conn_str(&dburl).await?;
            println!("Migrations completed");
        }
        other => {
            eprintln!("Unsupported backend '{}'. Supported: sqlite", other);
            std::process::exit(2);
        }
    }

    Ok(())
}
