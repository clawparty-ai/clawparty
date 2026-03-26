#!/bin/bash
#
# run-tests.sh - Run all tests and generate report
#
# Description:
#   1. Run API tests (Hub + Agent Core + Apps)
#   2. Run CLI tests
#   3. Generate test report
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPORT_DIR="$SCRIPT_DIR/reports"

# Generate timestamp
TIMESTAMP=$(date +"%Y%m%d%H%M%S")
LOG_FILE="$SCRIPT_DIR/test-result-$TIMESTAMP.log"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test statistics
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0
TEST_SUITES=()

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
    echo "[INFO] $1" >> "$LOG_FILE"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
    echo "[WARN] $1" >> "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    echo "[ERROR] $1" >> "$LOG_FILE"
}

log_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
    echo "[TEST] $1" >> "$LOG_FILE"
}

log_result() {
    local result=$1
    local name=$2
    local duration=$3
    echo "[$result] $name (${duration}s)" >> "$LOG_FILE"
}

# Initialize log file
init_log() {
    mkdir -p "$REPORT_DIR"
    echo "==========================================" > "$LOG_FILE"
    echo "ClawParty Test Log" >> "$LOG_FILE"
    echo "Time: $(date)" >> "$LOG_FILE"
    echo "==========================================" >> "$LOG_FILE"
    echo "" >> "$LOG_FILE"
    
    log_info "Log file: $LOG_FILE"
    log_info "Report directory: $REPORT_DIR"
}

# Run single test script
run_test() {
    local test_script=$1
    local test_name=$(basename "$test_script")
    local start_time=$(date +%s)

    log_test "Starting test: $test_name"

    local output
    local exit_code=0
    
    set +e
    output=$("$test_script" 2>&1); exit_code=$?
    set -e
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}: $test_name (${duration}s)"
        log_result "PASS" "$test_name" "$duration"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    elif [ $exit_code -eq 99 ]; then
        echo -e "${YELLOW}⊘ SKIP${NC}: $test_name (${duration}s)"
        log_result "SKIP" "$test_name" "$duration"
        TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}: $test_name (${duration}s)"
        log_result "FAIL" "$test_name" "$duration"
        echo "  Error output: $output" >> "$LOG_FILE"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Run all tests in directory
run_tests_in_dir() {
    local dir=$1
    local name=$2

    if [ ! -d "$dir" ]; then
        log_warn "Directory does not exist: $dir"
        return 0
    fi

    echo ""
    echo "=========================================="
    log_info "Running $name tests..."
    echo "==========================================" | tee -a "$LOG_FILE"
    
    TEST_SUITES+=("$name")

    local test_files=$(find "$dir" -name "*.sh" -type f | grep -v run-tests.sh | sort)

    if [ -z "$test_files" ]; then
        log_warn "No test files found: $dir"
        return 0
    fi

    for test_file in $test_files; do
        # Skip run-tests.sh itself
        if [[ "$(basename "$test_file")" == "run-tests.sh" ]]; then
            continue
        fi
        run_test "$test_file" || true
    done

    echo ""
}

# Generate report
generate_report() {
    log_info "Generating test report..."
    
    # Calculate percentage
    local total=$((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))
    local pass_rate=0
    if [ $total -gt 0 ]; then
        pass_rate=$((TESTS_PASSED * 100 / total))
    fi

    # Generate Markdown report
    local md_report="$REPORT_DIR/test-report-$TIMESTAMP.md"
    cat > "$md_report" << EOF
# ClawParty Test Report

## Summary

| Metric | Value |
|--------|-------|
| Passed | $TESTS_PASSED |
| Failed | $TESTS_FAILED |
| Skipped | $TESTS_SKIPPED |
| Total | $total |
| Pass Rate | $pass_rate% |

## Test Suites

$(for suite in "${TEST_SUITES[@]}"; do echo "- $suite"; done)

## Test Log

\`\`\`
$(cat "$LOG_FILE")
\`\`\`

## Generation Info

- Generated at: $(date)
- Log file: $LOG_FILE
- Markdown report: $md_report
EOF

    echo ""
    echo "=========================================="
    log_info "Report generated:"
    echo "  - Markdown: $md_report"
    echo "  - Log: $LOG_FILE"
    echo "=========================================="
}

# Print test summary
print_summary() {
    local total=$((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))
    local pass_rate=0
    if [ $total -gt 0 ]; then
        pass_rate=$((TESTS_PASSED * 100 / total))
    fi

    echo ""
    echo "=========================================="
    echo "Test Summary"
    echo "=========================================="
    echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
    echo -e "Skipped: ${YELLOW}$TESTS_SKIPPED${NC}"
    echo "Total: $total"
    echo "Pass rate: $pass_rate%"
    echo "=========================================="

    echo "" >> "$LOG_FILE"
    echo "==========================================" >> "$LOG_FILE"
    echo "Test Summary" >> "$LOG_FILE"
    echo "==========================================" >> "$LOG_FILE"
    echo "Passed: $TESTS_PASSED" >> "$LOG_FILE"
    echo "Failed: $TESTS_FAILED" >> "$LOG_FILE"
    echo "Skipped: $TESTS_SKIPPED" >> "$LOG_FILE"
    echo "Total: $total" >> "$LOG_FILE"
    echo "Pass rate: $pass_rate%" >> "$LOG_FILE"
    echo "==========================================" >> "$LOG_FILE"

    if [ $TESTS_FAILED -gt 0 ]; then
        log_error "Some tests failed!"
        exit 1
    else
        log_info "All tests passed!"
        exit 0
    fi
}

# Main function
main() {
    echo "=========================================="
    echo "ClawParty Test Suite"
    echo "=========================================="

    # Initialize log
    init_log

    # Run API tests
    run_tests_in_dir "$SCRIPT_DIR/api" "API"

    # Run CLI tests
    run_tests_in_dir "$SCRIPT_DIR/cli" "CLI"

    # Generate report
    generate_report

    # Print summary
    print_summary
}

main "$@"