#!/bin/bash

# Test script for TUI mode
set -e

echo "Testing TUI mode fix..."
echo "Starting connector with TUI..."

# Start the connector with TUI in a subshell, redirect output to log
(./run-node.sh -n "tui-test" > /tmp/tui-test.log 2>&1) &
TEST_PID=$!

# Wait for startup
sleep 5

# Check if the process is still running
if ps -p $TEST_PID > /dev/null 2>&1; then
    echo "✓ Process is running"

    # Check for the error in the log
    if grep -q "Failed to initialize input reader" /tmp/tui-test.log; then
        echo "✗ ERROR: TUI input reader initialization failed"
        kill $TEST_PID 2>/dev/null || true
        exit 1
    else
        echo "✓ No TUI input reader errors found"
    fi

    # Kill the test process
    kill $TEST_PID 2>/dev/null || true
    wait $TEST_PID 2>/dev/null || true

    echo ""
    echo "✓ TUI mode test PASSED"
    echo ""
    echo "Last 20 lines of log:"
    tail -20 /tmp/tui-test.log
    exit 0
else
    echo "✗ Process died unexpectedly"
    echo ""
    echo "Log output:"
    cat /tmp/tui-test.log
    exit 1
fi
