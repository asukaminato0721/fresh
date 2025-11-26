#!/usr/bin/env python3

import argparse
import re
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Literal, Optional

# ANSI color codes
RED = "\033[0;31m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
BLUE = "\033[0;34m"
NC = "\033[0m"

BumpType = Literal["patch", "minor", "major"]

def print_usage():
    """Prints the usage instructions."""
    print("Usage: ./bump-version.py [patch|minor|major]")
    print("")
    print("Examples:")
    print("  ./bump-version.py          # Bump patch version (default): 0.1.0 -> 0.1.1")
    print("  ./bump-version.py patch    # Bump patch version: 0.1.0 -> 0.1.1")
    print("  ./bump-version.py minor    # Bump minor version: 0.1.0 -> 0.2.0")
    print("  ./bump-version.py major    # Bump major version: 0.1.0 -> 1.0.0")
    print("")
    print("The script will:")
    print("  1. Read current version from Cargo.toml")
    print("  2. Calculate the new version")
    print("  3. Ask for confirmation")
    print("  4. Update Cargo.toml and Cargo.lock")
    print("  5. Optionally generate release notes")
    print("  6. Ask to commit, tag, and push the changes")

def run_command(command: List[str], capture_output: bool = False, check: bool = True) -> subprocess.CompletedProcess:
    """Runs a shell command."""
    return subprocess.run(command, capture_output=capture_output, text=True, check=check)

def get_current_version(cargo_toml_path: Path) -> str:
    """Gets the current version from Cargo.toml."""
    content = cargo_toml_path.read_text()
    match = re.search(r'^version = "(.*)"', content, re.MULTILINE)
    if not match:
        raise ValueError("Could not find version in Cargo.toml")
    return match.group(1)

def calculate_new_version(current_version: str, bump_type: BumpType) -> str:
    """Calculates the new version."""
    major, minor, patch = map(int, current_version.split("-")[0].split("."))
    if bump_type == "patch":
        patch += 1
    elif bump_type == "minor":
        minor += 1
        patch = 0
    elif bump_type == "major":
        major += 1
        minor = 0
        patch = 0
    return f"{major}.{minor}.{patch}"

def update_cargo_toml(cargo_toml_path: Path, current_version: str, new_version: str) -> None:
    """Updates the version in Cargo.toml."""
    content = cargo_toml_path.read_text()
    pattern = rf'^version = "{re.escape(current_version)}"'
    new_content = re.sub(
        pattern,
        f'version = "{new_version}"',
        content,
        count=1,
        flags=re.MULTILINE,
    )
    cargo_toml_path.write_text(new_content)

def update_cargo_lock() -> None:
    """Updates Cargo.lock by running cargo build."""
    try:
        run_command(["cargo", "build", "--quiet"])
    except subprocess.CalledProcessError as e:
        print(f"{YELLOW}Warning:{NC} cargo build had some output (this might be normal)")
        print(e.stderr)

def get_previous_tag() -> Optional[str]:
    """Gets the previous git tag."""
    try:
        result = run_command(["git", "describe", "--tags", "--abbrev=0"], capture_output=True)
        return result.stdout.strip()
    except subprocess.CalledProcessError:
        try:
            # If no tags, get the initial commit
            result = run_command(["git", "rev-list", "--max-parents=0", "HEAD"], capture_output=True)
            return result.stdout.strip()
        except subprocess.CalledProcessError:
            return None

def main() -> None:
    """Main function."""
    parser = argparse.ArgumentParser(description="Version bump script for the editor project.")
    parser.add_argument(
        "bump_type",
        nargs="?",
        default="patch",
        choices=["patch", "minor", "major"],
        help="The type of version bump.",
    )
    args = parser.parse_args()
    bump_type: BumpType = args.bump_type

    cargo_toml_path = Path("Cargo.toml")
    if not cargo_toml_path.exists():
        print(f"{RED}Error: Cargo.toml not found{NC}")
        print("Please run this script from the project root directory")
        sys.exit(1)

    try:
        current_version = get_current_version(cargo_toml_path)
    except ValueError as e:
        print(f"{RED}Error: {e}{NC}")
        sys.exit(1)

    new_version = calculate_new_version(current_version, bump_type)

    print(f"{BLUE}Version Bump ({bump_type}){NC}")
    print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
    print(f"Current version: {YELLOW}{current_version}{NC}")
    print(f"New version:     {GREEN}{new_version}{NC}")
    print("")

    reply = input(f"Bump {bump_type} version {current_version} -> {new_version}? (y/N) ").lower()
    if reply != "y":
        print("Aborted.")
        sys.exit(0)

    print("")
    print(f"{BLUE}Step 1:{NC} Updating Cargo.toml...")
    update_cargo_toml(cargo_toml_path, current_version, new_version)
    print(f"{GREEN}✓{NC} Updated Cargo.toml")

    print("")
    print(f"{BLUE}Step 2:{NC} Updating Cargo.lock (running cargo build)...")
    update_cargo_lock()
    print(f"{GREEN}✓{NC} Updated Cargo.lock")

    print("")
    print(f"{BLUE}Step 3:{NC} Summary of changes...")
    print("")
    try:
        diff_result = run_command(["git", "diff", "Cargo.toml", "Cargo.lock"], capture_output=True)
        print("Git diff:")
        print(diff_result.stdout)
    except subprocess.CalledProcessError:
        print("Could not get git diff.")


    print("")
    print(f"{GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{NC}")
    print("")



    print("")
    release_notes_content = ""
    release_notes_path = Path("RELEASE_NOTES.md")
    if release_notes_path.exists():
        release_notes_content = release_notes_path.read_text().strip()
        print(f"{BLUE}Found existing RELEASE_NOTES.md.{NC}")
    else:
        print(f"{YELLOW}Warning: RELEASE_NOTES.md not found. Tag will not include release notes.{NC}")

    reply = input(f"Commit, tag, and push v{new_version}? (y/N) ").lower()
    if reply != "y":
        print("")
        print(f"{YELLOW}Changes made but not committed.{NC}")
        print("")
        print("To complete manually:")
        print(f"  1. Commit changes: {YELLOW}git add Cargo.toml Cargo.lock && git commit -m 'Bump version to {new_version}'{NC}")
        if release_notes_content:
            print(f"  2. Create tag:     {YELLOW}git tag -a v{new_version} -F RELEASE_NOTES.md{NC}")
        else:
            print(f"  2. Create tag:     {YELLOW}git tag v{new_version}{NC}")
        print(f"  3. Push:           {YELLOW}git push && git push origin v{new_version}{NC}")
        sys.exit(0)

    try:
        current_branch_result = run_command(["git", "rev-parse", "--abbrev-ref", "HEAD"], capture_output=True)
        current_branch = current_branch_result.stdout.strip()
        
        print("")
        print(f"{BLUE}Step 4:{NC} Committing changes...")
        run_command(["git", "add", "Cargo.toml", "Cargo.lock"])
        run_command(["git", "commit", "-m", f"Bump version to {new_version}"])
        print(f"{GREEN}✓{NC} Committed")

        print("")
        print(f"{BLUE}Step 5:{NC} Creating tag v{new_version}...")
        if release_notes_content:
            run_command(["git", "tag", "-a", f"v{new_version}", "-F", "RELEASE_NOTES.md"])
        else:
            run_command(["git", "tag", f"v{new_version}"])
        print(f"{GREEN}✓{NC} Tagged")

        print("")
        print(f"{BLUE}Step 6:{NC} Pushing to origin...")
        run_command(["git", "push", "origin", current_branch])
        run_command(["git", "push", "origin", f"v{new_version}"])
        print(f"{GREEN}✓{NC} Pushed")

        print("")
        print(f"{GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{NC}")
        print(f"{GREEN}✓ Version {new_version} released!{NC}")
        print("")
        print("The GitHub Actions workflow will automatically create a release from the tag.")
        print("Once the GitHub Release action completes, you will need to manually publish the npm package:")
        print(f"  {YELLOW}npm publish https://github.com/sinelaw/fresh/releases/download/v{new_version}/fresh-editor-npm-package.tar.gz{NC}")

    except subprocess.CalledProcessError as e:
        print(f"{RED}An error occurred during git operations: {e}{NC}")
        print(e.stderr)
        sys.exit(1)
    except FileNotFoundError:
        print(f"{RED}Error: 'git' command not found. Is git installed and in your PATH?{NC}")
        sys.exit(1)

if __name__ == "__main__":
    main()
