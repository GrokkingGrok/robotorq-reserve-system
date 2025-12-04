use std::env;

/// Simple migration CLI for RoboTorq persistence backends.
///
/// Usage:
/// - `cargo run -p cargo-robotorq-migrate -- sqlite <DATABASE_URL> [TABLE_NAME]`
/// - Environment fallback: `DATABASE_URL` if not provided as an arg
/// - `TABLE_NAME` defaults to `_robotorq_migrations` when omitted
///
/// Examples:
/// - `cargo run -p cargo-robotorq-migrate -- sqlite sqlite://robotorq.db`
/// - `cargo run -p cargo-robotorq-migrate -- sqlite sqlite://file:memdb1?mode=memory&_foreign_keys=on _robotorq_migrations`

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1); // skip binary name

    let backend = args.next().unwrap_or_else(|| "sqlite".to_string());
    if backend == "--help" || backend == "-h" {
        print_usage();
        return Ok(());
    }

    match backend.as_str() {
        "sqlite" => {
            let dburl = args
                .next()
                .or_else(|| env::var("DATABASE_URL").ok())
                .unwrap_or_else(|| "sqlite://robotorq.db".to_string());
            let mig_table = args
                .next()
                .unwrap_or_else(|| "_robotorq_migrations".to_string());
            println!("Running sqlite migrations against: {dburl} (table: {mig_table})");
            commons::util::persistence::sqlite::run_migrations_with_conn_str(&dburl, &mig_table)
                .await?;
            println!("Migrations completed");
        }
        "postgres" => {
            let dburl = args
                .next()
                .or_else(|| env::var("DATABASE_URL").ok())
                .unwrap_or_else(|| {
                    "postgres://postgres:postgres@127.0.0.1:5432/postgres".to_string()
                });
            let mig_table = args
                .next()
                .unwrap_or_else(|| "_robotorq_migrations".to_string());
            println!("Running postgres migrations against: {dburl} (table: {mig_table})");
            run_postgres_migrations(&dburl, &mig_table).await?;
            println!("Migrations completed");
        }
        other => {
            eprintln!("Unsupported backend '{other}'. Supported: sqlite\n");
            print_usage();
            std::process::exit(2);
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!(
        "Usage:\n  cargo run -p cargo-robotorq-migrate -- sqlite <DATABASE_URL> [TABLE_NAME]\n\nArguments:\n  <DATABASE_URL>  Connection string (e.g., sqlite://robotorq.db)\n  [TABLE_NAME]    Optional migration tracking table (default: _robotorq_migrations)\n\nNotes:\n- If <DATABASE_URL> is omitted, the tool will use the DATABASE_URL environment variable.\n- Migrations are SQL files applied in order from crates/commons/sql/migrations, with each application recorded in the tracking table.\n- Recording includes filename, checksum, and applied_at to ensure idempotency and detect changes.\n\nExamples:\n  cargo run -p cargo-robotorq-migrate -- sqlite sqlite://robotorq.db\n  cargo run -p cargo-robotorq-migrate -- sqlite sqlite://file:memdb1?mode=memory&_foreign_keys=on _robotorq_migrations\n"
    );
}

// Minimal Postgres migration runner mirroring commons behavior
async fn run_postgres_migrations(
    url: &str,
    table_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use sqlx::PgPool;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    let pool = PgPool::connect(url).await?;

    // Ensure migrations tracking table exists
    let create_stmt = format!(
        "CREATE TABLE IF NOT EXISTS {} (filename TEXT PRIMARY KEY, checksum TEXT NOT NULL, applied_at BIGINT NOT NULL);",
        table_name
    );
    sqlx::query(&create_stmt).execute(&pool).await?;

    // Locate migrations directory relative to commons crate
    let mut migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    migrations_dir.pop(); // up from cargo-robotorq-migrate
    migrations_dir.pop(); // up to crates
    migrations_dir.push("commons");
    migrations_dir.push("sql");
    migrations_dir.push("migrations");

    if !migrations_dir.exists() {
        println!("No migrations directory at {}", migrations_dir.display());
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(&migrations_dir)?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|s| s == "sql"))
        .collect();
    entries.sort_by_key(std::fs::DirEntry::path);

    for ent in entries {
        let path = ent.path();
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid migration filename")?
            .to_string();
        let sql = fs::read_to_string(&path)?;
        let checksum = blake3::hash(sql.as_bytes()).to_hex().to_string();

        // Check existing
        let existing_query = format!("SELECT checksum FROM {} WHERE filename = $1", table_name);
        let existing: Option<(String,)> = sqlx::query_as(&existing_query)
            .bind(&filename)
            .fetch_optional(&pool)
            .await?;
        if let Some((existing_checksum,)) = existing {
            if existing_checksum == checksum {
                continue;
            }
            return Err(format!(
                "migration '{}' checksum mismatch (applied={} file={})",
                filename, existing_checksum, checksum
            )
            .into());
        }

        // Apply in transaction
        let mut conn = pool.acquire().await?;
        sqlx::query("BEGIN;").execute(&mut *conn).await?;
        if let Err(e) = sqlx::query(&sql).execute(&mut *conn).await {
            let _ = sqlx::query("ROLLBACK;").execute(&mut *conn).await;
            return Err(Box::new(e));
        }

        let applied_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let insert_stmt = format!(
            "INSERT INTO {}(filename, checksum, applied_at) VALUES ($1, $2, $3);",
            table_name
        );
        if let Err(e) = sqlx::query(&insert_stmt)
            .bind(&filename)
            .bind(&checksum)
            .bind(applied_at)
            .execute(&mut *conn)
            .await
        {
            let _ = sqlx::query("ROLLBACK;").execute(&mut *conn).await;
            return Err(Box::new(e));
        }
        sqlx::query("COMMIT;").execute(&mut *conn).await?;
    }

    Ok(())
}
