#!/bin/bin/env bash

# Pre-commit script for Rust. Requires nightly rust toolchain.

# Colors
RED='\033[0;1;31m'
GREEN='\033[0;1;32m'
YELLOW='\033[0;1;33m'
RESET='\033[0m'
BOLD='\033[0;1m'

# Git Metadata
ROOT_DIR="$(git rev-parse --show-toplevel)"
BUILD_DIR="${ROOT_DIR}/target"
BRANCH_NAME=$(git branch | grep '\*' | sed 's/* //')
STASH_NAME="pre-commit-$(date +%s) on ${BRANCH_NAME}"

echo "[*] ${BOLD}Checking for unstashed changes:${RESET}"
stash=0
# Check to make sure commit isn't empty
if git diff-index --cached --quiet HEAD --; then
    # It was empty, exit with status 0 to let git handle it
    exit 0
else
    # Stash changes that aren't added to the staging index so we test
    # only the changes to be committed
    old_stash=$(git rev-parse -q --verify refs/stash)
    git stash push -q --keep-index -m "$STASH_NAME"
    new_stash=$(git rev-parse -q --verify refs/stash)

    echo "[*] Stashed changes as: ${BOLD}${STASH_NAME}${RESET}"
    if [ "$old_stash" = "$new_stash" ]; then
        echo "[?] No changes, ${YELLOW}skipping tests${RESET}"
        exit 0
    else
        stash=1
    fi
fi

echo "[*] ${BOLD}Testing:${RESET}"
git diff --cached --stat
echo ""

## Local CI tasks to ensure all code quality checks pass
# Set a temporary alias to ensure the nightly toolchain is used
alias cargo='cargo +nightly'
    cargo fmt --all -- --check && \
    cargo clippy -- --D warnings && \
    cargo test --locked && \
    cargo doc --no-deps --document-private-items --all-features --workspace --verbose
# Remove the temporary alias for cargo to avoid messing with the user's system
unalias cargo

# Capture exit code from tests
status=$?

# Inform user of build failure
echo -ne "[*] ${BOLD}Build status:${RESET}"
if [ "$status" -ne "0" ]
then
    echo -ne "${RED}FAILED${RESET}\nTo commit your changes anyway, use ${BOLD}'--no-verify'${RESET}\n"
else
    echo -ne "${GREEN}PASSED${RESET}\n"
fi

# Revert stash if changes were stashed to restore working directory files
if [ "$stash" -eq 1 ]
then
    echo -ne "[*] ${BOLD}Restoring working tree${RESET} ...\n"
    if git reset --hard -q &&
       git stash apply --index -q &&
       git stash drop -q
    then
        echo -ne "\t${GREEN}restored${RESET} ${STASH_NAME}\n"
    else
        echo -ne "\t${RED}failed to revert git stash command${RESET}\n"
    fi
fi

# Exit with exit code from tests, so if they fail, prevent commit
exit $status