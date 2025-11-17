#!/usr/bin/env python3
"""
Label GitHub Issues for Phase 5+ Organization
Requires: pip install PyGithub
Set environment variable: GITHUB_TOKEN
"""

import os
from github import Github

# Configuration
REPO_OWNER = "GrokkingGrok"
REPO_NAME = "robotorq-network"

# Labels to create/use
LABELS = {
    "phase-6": {"color": "0E8A16", "description": "Future work for Phase 6"},
    "architecture": {"color": "1D76DB", "description": "Architectural improvements"},
    "monitoring": {"color": "FBCA04", "description": "Observability and metrics"},
    "future-enhancement": {"color": "BFD4F2", "description": "Future improvements"}
}

# Issue labeling map
ISSUE_LABELS = {
    # Phase 6 Crypto Work
    71: ["phase-6", "architecture"],  # POST-QUANTUM CRYPTOGRAPHY
    72: ["phase-6", "future-enhancement"],  # Replace with real Dilithium5
    73: ["phase-6", "future-enhancement"],  # Implement Dilithium5 verification
    75: ["phase-6", "architecture"],  # Add robot identity tracking
    76: ["phase-6", "future-enhancement"],  # Replace with robot registry lookup
    77: ["phase-6", "future-enhancement"],  # Add robot identity to JTU
    78: ["phase-6", "future-enhancement"],  # Falcon-1024
    79: ["phase-6", "future-enhancement"],  # Replace with real Falcon-1024
    80: ["phase-6", "future-enhancement"],  # Replace with Falcon-1024!
    81: ["phase-6", "future-enhancement"],  # Wire this up!
    
    # Monitoring
    82: ["monitoring"],  # Register with Prometheus registry
    83: ["monitoring"],  # Register with Prometheus registry
    
    # Production
    84: ["phase-6", "future-enhancement"],  # for Production
    88: ["phase-6", "future-enhancement"],  # for Production
    
    # Phase 5 Remaining Work
    92: ["phase-6", "future-enhancement"],  # Implement slashing
    93: ["phase-6", "future-enhancement"],  # Add integration test with real Falcon
    94: ["phase-6", "future-enhancement"],  # Remove fallback once all Diggers send signatures
    
    # Architecture/Refactor
    59: ["architecture"],  # comments
    60: ["phase-6", "architecture"],  # COMPLETE REWRITE NEEDED
    61: ["architecture", "future-enhancement"],  # Test needs refactoring
    62: ["architecture"],  # See why flagged as warning
    63: ["architecture", "future-enhancement"],  # Tests need refactoring
    64: ["architecture"],  # See why flagged as warning
    65: ["architecture", "future-enhancement"],  # Consider shared models package
    66: ["architecture"],  # File replaces old RoboTorqBatch concept
    67: ["phase-6", "architecture"],  # Add after proof archive implemented
    68: ["phase-6", "architecture"],  # Implement after ingots contain Units[]
    69: ["future-enhancement"],  # Use proper UUID library
    70: ["phase-6", "architecture"],  # Add proof archive integration
    74: ["phase-6", "architecture"],  # Future ledger integration
}


def main():
    # Get GitHub token from environment
    token = os.getenv("GITHUB_TOKEN")
    if not token:
        print("❌ Error: GITHUB_TOKEN environment variable not set")
        print("\nTo set it:")
        print("  PowerShell: $env:GITHUB_TOKEN='your_token_here'")
        print("  Linux/Mac:  export GITHUB_TOKEN='your_token_here'")
        print("\nGet a token from: https://github.com/settings/tokens")
        return 1
    
    # Connect to GitHub
    print(f"🔗 Connecting to GitHub...")
    g = Github(token)
    repo = g.get_repo(f"{REPO_OWNER}/{REPO_NAME}")
    print(f"✅ Connected to {REPO_OWNER}/{REPO_NAME}\n")
    
    # Create labels if they don't exist
    print("📋 Creating labels...")
    existing_labels = {label.name: label for label in repo.get_labels()}
    
    for label_name, label_config in LABELS.items():
        if label_name in existing_labels:
            # Update existing label
            label = existing_labels[label_name]
            label.edit(label_name, label_config["color"], label_config["description"])
            print(f"  ✏️  Updated: {label_name}")
        else:
            # Create new label
            repo.create_label(label_name, label_config["color"], label_config["description"])
            print(f"  ✨ Created: {label_name}")
    
    print()
    
    # Label issues
    print("🏷️  Labeling issues...")
    success_count = 0
    error_count = 0
    
    for issue_number, labels in ISSUE_LABELS.items():
        try:
            issue = repo.get_issue(issue_number)
            
            # Get current labels
            current_labels = [label.name for label in issue.labels]
            
            # Add new labels (preserve existing ones)
            all_labels = list(set(current_labels + labels))
            
            # Update issue
            issue.set_labels(*all_labels)
            print(f"  ✅ Issue #{issue_number}: {', '.join(labels)}")
            success_count += 1
            
        except Exception as e:
            print(f"  ❌ Issue #{issue_number}: {e}")
            error_count += 1
    
    print()
    print("="*60)
    print(f"✨ Complete! Labeled {success_count}/{len(ISSUE_LABELS)} issues")
    if error_count > 0:
        print(f"⚠️  {error_count} errors occurred")
    print(f"\n🔗 View issues: https://github.com/{REPO_OWNER}/{REPO_NAME}/issues")
    
    return 0 if error_count == 0 else 1


if __name__ == "__main__":
    exit(main())
