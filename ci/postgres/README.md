Postgres Docker Compose (for local Testcontainers / manual testing)

This folder contains a simple `docker-compose.yml` suitable for local development and for running the commons Postgres integration tests when Docker is available.

Notes
- The service uses `postgres:15` and exposes `5432` on the host.
- Credentials are `postgres` / `postgres` and the default DB is `postgres` to match Testcontainers-based tests in the repo.
- This compose file is intentionally minimal – tests in the repo use Testcontainers or a direct connection URL. Use this compose setup when you want a local, long-running Postgres instance for manual testing.

Start the Postgres container

Bash / WSL / macOS / Linux:

```bash
cd ci/postgres
docker compose up -d
```

Windows PowerShell (one-liner, no script file required):

```powershell
cd ci\postgres; docker compose up -d
```

Stop and remove (including data volume):

```bash
cd ci/postgres
docker compose down -v
```

How to run the repository Postgres tests against this local DB

- Set the project to use Postgres as the persistence backend and point to the DB URL. Example (PowerShell):

```powershell
$env:DATABASE_URL = "postgres://postgres:postgres@127.0.0.1:5432/postgres"
$env:RTQ_ENABLE_TESTCONTAINERS = "0"  # Tests that use Testcontainers will skip; using this manual DB
cargo test -p commons --features "persistence-postgres" --test persistence_postgres_config_verification
```

- If you prefer Testcontainers (container-per-test behavior), use the repo's Testcontainers tests by enabling `persistence-testcontainers` and ensuring Docker is running. When running Testcontainers tests, set:

```powershell
# enable container tests
$env:RTQ_ENABLE_TESTCONTAINERS = "1"
cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test readyz_postgres_integration
```

Troubleshooting
- If `docker compose up` fails, ensure Docker Desktop or your Docker engine is running and that you have permissions to bind port 5432.
- If a local Postgres instance is already listening on 5432, either stop it or change the host binding in `docker-compose.yml` (e.g., `"5433:5432"`) and use the adjusted port in the `DATABASE_URL`.

Diagnostics
If `docker compose up` appears to hang, use these diagnostic commands to see what Docker and the Postgres service are doing. Open two shells: one to run `docker compose up` in the foreground, and a second to run the inspection commands.

Run compose in the foreground to watch startup logs (Bash):
```bash
cd ci/postgres
docker compose up
```

Run compose in the foreground to watch startup logs (PowerShell):
```powershell
cd ci\postgres
docker compose up
```

In a second shell, inspect service/container state and logs (Bash):
```bash
docker compose ps
docker ps --filter name=robotorq-postgres
docker compose logs postgres --follow
docker inspect --format '{{json .State}}' robotorq-postgres
docker logs robotorq-postgres --tail 200
# Check for processes listening on TCP 5432
ss -ltnp | grep ':5432' || true
```

In a second shell, inspect service/container state and logs (PowerShell):
```powershell
docker compose ps
docker ps --filter name=robotorq-postgres
docker compose logs postgres --follow
docker inspect --format '{{json .State}}' robotorq-postgres | ConvertFrom-Json
docker logs robotorq-postgres --tail 200
# Find processes listening on 5432
netstat -ano | Select-String ":5432"
# If a PID is reported, inspect it
# tasklist /FI "PID eq <PID>"
```

What to look for
- If the container repeatedly restarts or shows `initdb` errors in logs, inspect volume permissions or the `docker compose` output for errors.
- If logs show `database system is ready to accept connections` but the healthcheck still fails, the healthcheck may be too strict — try increasing retries or simplifying the test (see next section).
- If `docker compose up` fails immediately with bind errors, choose a different host port mapping or stop the local service occupying the port.

Security note
- This compose file is for local development only. Do not use these credentials or the exposed port settings in a production environment.
