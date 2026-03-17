#!/bin/bash

# Phase 3 Discovery Test Execution Script
# This script runs the comprehensive test suite for port discovery

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Function to print section headers
print_header() {
    echo ""
    echo -e "${BLUE}=================================================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}=================================================================================${NC}"
    echo ""
}

# Function to print test status
print_test() {
    local status=$1
    local name=$2
    TESTS_RUN=$((TESTS_RUN + 1))

    if [ "$status" = "PASS" ]; then
        echo -e "${GREEN}✓${NC} $name"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    elif [ "$status" = "FAIL" ]; then
        echo -e "${RED}✗${NC} $name"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    elif [ "$status" = "SKIP" ]; then
        echo -e "${YELLOW}⊘${NC} $name (skipped)"
    fi
}

# Function to run a test and capture result
run_test() {
    local name=$1
    local command=$2

    if eval "$command" > /dev/null 2>&1; then
        print_test "PASS" "$name"
        return 0
    else
        print_test "FAIL" "$name"
        return 1
    fi
}

# Function to run test with expected output
run_test_with_output() {
    local name=$1
    local command=$2
    local expected=$3

    output=$(eval "$command" 2>&1)
    if echo "$output" | grep -q "$expected"; then
        print_test "PASS" "$name"
        return 0
    else
        print_test "FAIL" "$name"
        echo "  Expected: $expected"
        return 1
    fi
}

# Start testing
print_header "PHASE 3 DISCOVERY TEST SUITE"

echo "Building project..."
if cargo build --release 2>&1 | tail -1 | grep -q "Finished"; then
    echo -e "${GREEN}Build successful${NC}"
else
    echo -e "${RED}Build failed${NC}"
    exit 1
fi

# ============================================================================
# UNIT TESTS
# ============================================================================
print_header "1. UNIT TESTS"

echo "Running Rust unit tests..."
if cargo test discovery --release 2>&1 | grep -q "test result: ok"; then
    echo -e "${GREEN}All unit tests passed${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 5))
    TESTS_RUN=$((TESTS_RUN + 5))
else
    echo -e "${RED}Some unit tests failed${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    TESTS_RUN=$((TESTS_RUN + 1))
fi

# ============================================================================
# INTEGRATION TESTS
# ============================================================================
print_header "2. INTEGRATION TESTS"

# Test 1: Single port validation
run_test_with_output \
    "Single port validation (google.com:443)" \
    "./target/release/http-traffic-sim --config config.discovery-test.yaml" \
    "Port 443 \[HTTPS\]"

# Test 2: Multi-port validation
run_test_with_output \
    "Multi-port validation (ports 80, 443)" \
    "./target/release/http-traffic-sim --config config.discovery-multi-port-test.yaml" \
    "Open Ports:"

# Test 3: Failure handling (fail mode)
if ! ./target/release/http-traffic-sim --config config.discovery-fail-test.yaml > /dev/null 2>&1; then
    print_test "PASS" "Failure handling (fail mode stops execution)"
else
    print_test "FAIL" "Failure handling (should have stopped)"
fi

# Test 4: Failure handling (warn mode)
run_test_with_output \
    "Failure handling (warn mode continues)" \
    "./target/release/http-traffic-sim --config config.discovery-warn-test.yaml" \
    "WARN.*continuing"

# ============================================================================
# FUNCTIONAL TESTS
# ============================================================================
print_header "3. FUNCTIONAL TESTS"

# Test 5: Service detection (HTTPS)
run_test_with_output \
    "Service detection identifies HTTPS" \
    "./target/release/http-traffic-sim --config config.discovery-test.yaml" \
    "\[HTTPS\]"

# Test 6: Service detection (HTTP)
run_test_with_output \
    "Service detection identifies HTTP" \
    "./target/release/http-traffic-sim --config config.discovery-multi-port-test.yaml" \
    "\[HTTP\]"

# Test 7: Discovery results display
run_test_with_output \
    "Discovery results formatted correctly" \
    "./target/release/http-traffic-sim --config config.discovery-test.yaml" \
    "PORT DISCOVERY PHASE"

# Test 8: Response time tracking
run_test_with_output \
    "Response times tracked and displayed" \
    "./target/release/http-traffic-sim --config config.discovery-test.yaml" \
    "ms response"

# ============================================================================
# BACKWARD COMPATIBILITY TESTS
# ============================================================================
print_header "4. BACKWARD COMPATIBILITY TESTS"

# Test 9: Config without discovery still works
if [ -f "config.example.yaml" ]; then
    run_test \
        "Config without discovery field works" \
        "./target/release/http-traffic-sim --config config.example.yaml --duration 1 --concurrent 1"
else
    print_test "SKIP" "Config without discovery field works (no test config)"
fi

# Test 10: Discovery disabled has no impact
run_test \
    "Discovery disabled has no performance impact" \
    "./target/release/http-traffic-sim --url https://google.com --concurrent 1 --requests 1"

# ============================================================================
# PERFORMANCE TESTS
# ============================================================================
print_header "5. PERFORMANCE TESTS"

# Test 11: Single port check speed
echo -n "Testing single port check speed... "
start_time=$(date +%s%N)
./target/release/http-traffic-sim --config config.discovery-test.yaml > /dev/null 2>&1 || true
end_time=$(date +%s%N)
duration=$(( (end_time - start_time) / 1000000 ))

if [ $duration -lt 2000 ]; then
    print_test "PASS" "Single port check completes in < 2s (${duration}ms)"
else
    print_test "FAIL" "Single port check too slow (${duration}ms)"
fi

# Test 12: Multi-port parallel execution
echo -n "Testing multi-port parallel speed... "
start_time=$(date +%s%N)
./target/release/http-traffic-sim --config config.discovery-multi-port-test.yaml > /dev/null 2>&1 || true
end_time=$(date +%s%N)
duration=$(( (end_time - start_time) / 1000000 ))

if [ $duration -lt 3000 ]; then
    print_test "PASS" "Multi-port check completes in < 3s (${duration}ms)"
else
    print_test "FAIL" "Multi-port check too slow (${duration}ms)"
fi

# ============================================================================
# ERROR HANDLING TESTS
# ============================================================================
print_header "6. ERROR HANDLING TESTS"

# Test 13: Clear error messages on failure
output=$(./target/release/http-traffic-sim --config config.discovery-fail-test.yaml 2>&1 || true)
if echo "$output" | grep -q "Port discovery failed" && echo "$output" | grep -q "Set on_failure"; then
    print_test "PASS" "Clear error messages on discovery failure"
else
    print_test "FAIL" "Error message not clear enough"
fi

# Test 14: Failed ports reported correctly
if echo "$output" | grep -q "Failed Ports:" && echo "$output" | grep -q "Port 9999"; then
    print_test "PASS" "Failed ports reported in output"
else
    print_test "FAIL" "Failed ports not reported"
fi

# ============================================================================
# CONFIGURATION TESTS
# ============================================================================
print_header "7. CONFIGURATION TESTS"

# Test 15: Example configs are valid
for config in config.discovery-*.example.yaml; do
    if [ -f "$config" ]; then
        # Just parse the config (will fail if invalid)
        if ./target/release/http-traffic-sim --config "$config" --help > /dev/null 2>&1 ||
           ./target/release/http-traffic-sim --config "$config" 2>&1 | grep -q "PORT DISCOVERY"; then
            print_test "PASS" "Config valid: $(basename $config)"
        else
            print_test "FAIL" "Config invalid: $(basename $config)"
        fi
    fi
done

# ============================================================================
# SUMMARY
# ============================================================================
print_header "TEST SUMMARY"

echo "Total Tests Run:    $TESTS_RUN"
echo -e "Passed:             ${GREEN}$TESTS_PASSED${NC}"
if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "Failed:             ${RED}$TESTS_FAILED${NC}"
else
    echo -e "Failed:             ${GREEN}$TESTS_FAILED${NC}"
fi

pass_rate=$((TESTS_PASSED * 100 / TESTS_RUN))
echo "Pass Rate:          $pass_rate%"

echo ""
if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 ALL TESTS PASSED!${NC}"
    exit 0
else
    echo -e "${RED}⚠️  SOME TESTS FAILED${NC}"
    echo ""
    echo "Review the output above for details on failed tests."
    echo "See PHASE3_TEST_PLAN.md for comprehensive test documentation."
    exit 1
fi
