#!/usr/bin/env python3
"""
Refinery Logs Viewer
Streams logs from the Refinery container in real-time
"""

import subprocess
import sys
import argparse

CONTAINER_NAME = "robotorq-network-refinery-1"

def main():
    parser = argparse.ArgumentParser(description="Stream Refinery container logs")
    parser.add_argument(
        "--tail",
        type=int,
        default=50,
        help="Number of lines to show from the end (default: 50)"
    )
    parser.add_argument(
        "-f", "--follow",
        action="store_true",
        help="Follow log output (stream continuously)"
    )
    args = parser.parse_args()
    tail_lines = str(args.tail)
    # Check if container is running
    try:
        result = subprocess.run(
            ["docker", "ps", "--filter", f"name={CONTAINER_NAME}", "--format", "{{.Names}}"],
            capture_output=True,
            text=True,
            check=True
        )
        if CONTAINER_NAME not in result.stdout:
            print(f"❌ Container {CONTAINER_NAME} is not running")
            sys.exit(1)
    except subprocess.CalledProcessError:
        print("❌ Failed to check container status")
        sys.exit(1)
    
    # Stream logs
    if args.follow:
        print(f"📋 Streaming logs from {CONTAINER_NAME} (last {tail_lines} lines, following)...")
        print(f"    Press Ctrl+C to stop\n")
    else:
        print(f"📋 Showing last {tail_lines} lines from {CONTAINER_NAME}...\n")
    
    try:
        cmd = ["docker", "logs", "--tail", tail_lines, CONTAINER_NAME]
        if args.follow:
            cmd.insert(2, "-f")
        subprocess.run(cmd)
    except KeyboardInterrupt:
        print("\n\n✅ Stopped streaming logs")
        sys.exit(0)

if __name__ == "__main__":
    main()
