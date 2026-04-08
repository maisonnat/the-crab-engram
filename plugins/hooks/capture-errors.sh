#!/bin/bash
# engram error capture hook
# Parses cargo build/test output and captures errors as observations
#
# Usage: cargo test 2>&1 | plugins/hooks/capture-errors.sh
# Or:    cargo build 2>&1 | plugins/hooks/capture-errors.sh

PROJECT=$(basename "$(git rev-parse --show-toplevel 2>/dev/null || echo 'default')")
SESSION_ID="${ENGRAM_SESSION_ID:-$(uuidgen 2>/dev/null || date +%s)}"

# Read stdin and capture errors
ERROR_FOUND=0
ERROR_MSG=""
ERROR_FILE=""
STACK_TRACE=""

while IFS= read -r line; do
    echo "$line"  # Pass through to stdout

    # Detect compilation errors
    if echo "$line" | grep -qE "^error\[E[0-9]+\]"; then
        ERROR_FOUND=1
        ERROR_MSG="$line"
        ERROR_TYPE="compile_error"
    fi

    # Detect test failures
    if echo "$line" | grep -qE "FAILED|panicked at"; then
        ERROR_FOUND=1
        ERROR_MSG="$line"
        ERROR_TYPE="test_failure"
    fi

    # Detect file:line references
    if echo "$line" | grep -qE "^\s*-->.+:[0-9]+:[0-9]+"; then
        ERROR_FILE=$(echo "$line" | sed 's/.*--> //' | sed 's/:.*//')
    fi

    # Accumulate stack trace
    if [ "$ERROR_FOUND" = "1" ]; then
        STACK_TRACE="${STACK_TRACE}${line}\n"
    fi
done

# If we found an error, capture it
if [ "$ERROR_FOUND" = "1" ] && [ -n "$ERROR_MSG" ]; then
    engram capture-error \
        --error-type "${ERROR_TYPE:-unknown}" \
        --error-message "$ERROR_MSG" \
        --file-line "$ERROR_FILE" \
        --stack-trace "$(echo -e "$STACK_TRACE" | tail -20)" \
        --session-id "$SESSION_ID" \
        --project "$PROJECT" \
        2>/dev/null || true
fi
