#!/bin/bash
# Organize test results for PRD 6 Memory Module

TEST_DIR="tests/memory"
REPORT_FILE="${TEST_DIR}/test_report.md"

echo "Memory Module Test Report" > "$REPORT_FILE"
echo "=========================" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "Generated: $(date)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Run tests and capture results
echo "Running memory tests..." >> "$REPORT_FILE"
cargo test --test memory --no-fail-fast 2>&1 | tee -a "$REPORT_FILE"

echo "" >> "$REPORT_FILE"
echo "Test organization complete."
