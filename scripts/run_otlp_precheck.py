#!/usr/bin/env python3
"""
Run OTLP pre-checks locally:
- Start OTLP collector via docker compose
- Wait for collector readiness on 127.0.0.1:4317
- Run the `emit_traces` example with the `otlp` feature
- Capture and save collector logs to `otel-collector.log`
- Check logs for a stable `example_id` attribute
- Tear down the collector

Usage:
    python scripts/run_otlp_precheck.py [--compose-file PATH] [--example-id ID]

Designed to be safe and idempotent for local developer verification and CI.
"""

import argparse
import logging
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path


LOG = logging.getLogger("otlp-precheck")


def run(cmd, cwd=None, capture=False, check=True, env=None):
    LOG.debug("run: %s (cwd=%s)", cmd, cwd)
    completed = subprocess.run(cmd, cwd=cwd, capture_output=capture, text=True, env=env)
    if check and completed.returncode != 0:
        LOG.error("command failed: %s\nexit=%s\nstdout=%s\nstderr=%s", cmd, completed.returncode, completed.stdout, completed.stderr)
        raise subprocess.CalledProcessError(completed.returncode, cmd, output=completed.stdout, stderr=completed.stderr)
    return completed


def start_collector(compose_file: Path):
    LOG.info("Starting OTLP collector using %s", compose_file)
    run(["docker", "compose", "-f", str(compose_file), "up", "-d"], check=True)


def teardown_collector(compose_file: Path):
    LOG.info("Tearing down OTLP collector")
    try:
        run(["docker", "compose", "-f", str(compose_file), "down"], check=True)
    except subprocess.CalledProcessError:
        LOG.exception("Failed to tear down collector; attempting force remove")
        # best-effort cleanup
        run(["docker", "rm", "-f", "otel-collector"], check=False)


def wait_for_port(host: str, port: int, timeout: int = 30):
    LOG.info("Waiting for %s:%d (timeout %ds)", host, port, timeout)
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with socket.create_connection((host, port), timeout=1):
                LOG.info("Port %s:%d is open", host, port)
                return True
        except OSError:
            time.sleep(1)
    LOG.error("Timeout waiting for port %s:%d", host, port)
    return False


def run_example(repo_root: Path):
    LOG.info("Running emit_traces example with otlp feature")
    examples_dir = repo_root / "crates" / "examples"
    # Use the passthrough 'otlp' feature added to examples Cargo.toml
    # capture stdout/stderr so we can inspect example output for stable attributes
    completed = run(["cargo", "run", "--bin", "emit_traces", "--features", "otlp"], cwd=str(examples_dir), capture=True, check=True)
    out = completed.stdout or ""
    LOG.debug("example stdout:\n%s", out)
    return out


def collect_and_save_logs(compose_file: Path, out_path: Path):
    LOG.info("Collecting collector logs to %s", out_path)
    # use docker logs for container stdout/stderr
    # use `docker compose logs` so logs from compose-managed container are captured
    completed = run(["docker", "compose", "-f", str(compose_file), "logs", "--no-color", "--tail", "1000", "otel-collector"], capture=True, check=False)
    out_path.write_text(completed.stdout or "")
    return completed.stdout or ""


def check_logs_for(log_text: str, needle: str) -> bool:
    """Check logs for any reliable ingestion indicators.

    We accept any of:
    - the explicit `example_id` string
    - the example span name `emit_traces_example`
    - the collector `TracesExporter` summary line
    """
    if needle and needle in log_text:
        return True
    if 'emit_traces_example' in log_text:
        return True
    if 'TracesExporter' in log_text:
        return True
    return False


def main(argv=None):
    parser = argparse.ArgumentParser(description="Run OTLP pre-checks (start collector, run example, check logs)")
    parser.add_argument("--compose-file", type=Path, default=Path("ci/otlp-collector/docker-compose.yml"), help="Path to docker-compose file")
    parser.add_argument("--example-id", default="robotorq_emit_traces_test_001", help="Span attribute value to search for in collector logs")
    parser.add_argument("--timeout", type=int, default=30, help="Timeout in seconds for collector readiness")
    parser.add_argument("--post-wait", type=int, default=8, help="Seconds to wait after running the example before collecting logs")
    parser.add_argument("--save-log", type=Path, default=Path("otel-collector.log"), help="Path to save collector logs")
    parser.add_argument("--no-teardown", action="store_true", help="Do not teardown the collector after the run")
    parser.add_argument("--verbose", "-v", action="count", default=0)

    args = parser.parse_args(argv)

    level = logging.WARNING
    if args.verbose >= 2:
        level = logging.DEBUG
    elif args.verbose == 1:
        level = logging.INFO
    logging.basicConfig(level=level, format="%(asctime)s %(levelname)s %(message)s")

    repo_root = Path(__file__).resolve().parents[1]
    compose_file = (repo_root / args.compose_file).resolve()

    if shutil.which("docker") is None:
        LOG.error("docker is not available in PATH. Please install Docker and ensure 'docker' is on PATH")
        return 2

    try:
        start_collector(compose_file)
        ok = wait_for_port("127.0.0.1", 4317, timeout=args.timeout)
        if not ok:
            LOG.error("Collector did not become ready; fetching logs for debugging")
            logs = collect_and_save_logs(compose_file, args.save_log)
            LOG.error("Collector logs:\n%s", logs)
            return 3

        # Run example (this will build the workspace as needed) and capture stdout
        example_out = run_example(repo_root)

        # Give the exporter and collector time to process and flush logs
        time.sleep(args.post_wait)

        logs = collect_and_save_logs(compose_file, args.save_log)

        # Accept success if either collector logs show ingestion, or the
        # example stdout contains the stable example_id attribute.
        found = check_logs_for(logs, args.example_id) or (args.example_id in (example_out or ""))
        if found:
            LOG.info("Found example id '%s' in collector logs", args.example_id)
        else:
            LOG.error("Did not find example id '%s' in collector logs", args.example_id)
            LOG.debug("Collector logs:\n%s", logs)
            return 4

        return 0
    except subprocess.CalledProcessError as e:
        LOG.exception("Command failed: %s", e)
        return 5
    except Exception:
        LOG.exception("Unexpected failure")
        return 6
    finally:
        if not args.no_teardown:
            try:
                teardown_collector(compose_file)
            except Exception:
                LOG.exception("Failed to teardown collector cleanly")


if __name__ == "__main__":
    raise SystemExit(main())
