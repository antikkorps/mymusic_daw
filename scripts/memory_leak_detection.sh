#!/bin/bash
# Memory leak detection script for MyMusic DAW
# This script runs tests with Valgrind and AddressSanitizer to detect memory leaks

set -e

echo "🔍 Memory Leak Detection for MyMusic DAW"
echo "=========================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Check if Valgrind is installed
check_valgrind() {
    if command -v valgrind &> /dev/null; then
        print_success "Valgrind is installed"
        return 0
    else
        print_error "Valgrind is not installed"
        print_warning "Install with: brew install valgrind (macOS) or apt-get install valgrind (Linux)"
        return 1
    fi
}

# Check if AddressSanitizer is available
check_asan() {
    if rustc --print target-features | grep -q sanitize; then
        print_success "AddressSanitizer is available"
        return 0
    else
        print_error "AddressSanitizer is not available"
        return 1
    fi
}

# Run tests with Valgrind
run_valgrind_tests() {
    echo ""
    echo "🧪 Running tests with Valgrind..."
    echo "=================================="
    
    # Build tests in debug mode
    print_warning "Building tests in debug mode..."
    cargo test --no-run
    
    # Find test executables
    TEST_BINARIES=$(find target/debug/deps -name "mymusic_daw-*" -type f -executable | head -5)
    
    if [ -z "$TEST_BINARIES" ]; then
        print_error "No test binaries found"
        return 1
    fi
    
    # Run Valgrind on each test binary
    for test_binary in $TEST_BINARIES; do
        echo ""
        echo "Testing: $(basename $test_binary)"
        echo "-----------------------------------"
        
        # Run with Valgrind
        valgrind --leak-check=full --show-leak-kinds=all --track-origins=yes --verbose --log-file=valgrind_$(basename $test_binary).log "$test_binary" || true
        
        # Check for leaks in the log
        if grep -q "definitely lost: 0 bytes in 0 blocks" valgrind_$(basename $test_binary).log; then
            print_success "No memory leaks detected in $(basename $test_binary)"
        else
            print_error "Memory leaks detected in $(basename $test_binary)"
            print_warning "Check valgrind_$(basename $test_binary).log for details"
        fi
    done
}

# Run tests with AddressSanitizer
run_asan_tests() {
    echo ""
    echo "🔬 Running tests with AddressSanitizer..."
    echo "=========================================="
    
    # Set environment variables for AddressSanitizer
    export RUSTFLAGS="-Z sanitizer=address"
    export ASAN_OPTIONS="detect_leaks=1:check_initialization_order=1:strict_init_order=1"
    
    # Run tests with AddressSanitizer
    print_warning "Running tests with AddressSanitizer (this may take a while)..."
    cargo +nightly test --target x86_64-apple-darwin 2>&1 | tee asan_output.log || true
    
    # Check for leaks in the output
    if grep -q "ERROR: LeakSanitizer: detected memory leaks" asan_output.log; then
        print_error "Memory leaks detected by AddressSanitizer"
        print_warning "Check asan_output.log for details"
    else
        print_success "No memory leaks detected by AddressSanitizer"
    fi
    
    # Unset environment variables
    unset RUSTFLAGS
    unset ASAN_OPTIONS
}

# Run specific memory-intensive tests
run_memory_intensive_tests() {
    echo ""
    echo "🧠 Running memory-intensive tests..."
    echo "===================================="
    
    # Test voice allocation/deallocation
    print_warning "Testing voice manager memory handling..."
    cargo test voice_manager -- --nocapture
    
    # Test plugin loading/unloading
    print_warning "Testing plugin system memory handling..."
    cargo test plugin -- --nocapture
    
    # Test project serialization/deserialization
    print_warning "Testing project persistence memory handling..."
    cargo test project_persistence -- --nocapture
}

# Generate memory usage report
generate_report() {
    echo ""
    echo "📊 Memory Usage Report"
    echo "======================"
    
    # Check if any Valgrind logs exist
    if ls valgrind_*.log 1> /dev/null 2>&1; then
        echo "Valgrind logs generated:"
        ls -la valgrind_*.log
        
        # Extract summary from each log
        for log in valgrind_*.log; do
            echo ""
            echo "Summary for $log:"
            grep -A5 "HEAP SUMMARY" "$log" || echo "No heap summary found"
        done
    fi
    
    # Check AddressSanitizer output
    if [ -f asan_output.log ]; then
        echo ""
        echo "AddressSanitizer output: asan_output.log"
        # Count errors
        LEAK_COUNT=$(grep -c "ERROR: LeakSanitizer: detected memory leaks" asan_output.log || echo "0")
        echo "Leaks detected: $LEAK_COUNT"
    fi
}

# Clean up test files
cleanup() {
    echo ""
    echo "🧹 Cleaning up test files..."
    rm -f valgrind_*.log asan_output.log
    print_success "Cleanup completed"
}

# Main execution
main() {
    echo "Starting memory leak detection..."
    
    # Check prerequisites
    VALGRIND_AVAILABLE=0
    ASAN_AVAILABLE=0
    
    if check_valgrind; then
        VALGRIND_AVAILABLE=1
    fi
    
    if check_asan; then
        ASAN_AVAILABLE=1
    fi
    
    # Run tests based on available tools
    if [ $VALGRIND_AVAILABLE -eq 1 ]; then
        run_valgrind_tests
    else
        print_warning "Skipping Valgrind tests (not available)"
    fi
    
    if [ $ASAN_AVAILABLE -eq 1 ]; then
        run_asan_tests
    else
        print_warning "Skipping AddressSanitizer tests (not available)"
    fi
    
    # Always run memory-intensive tests
    run_memory_intensive_tests
    
    # Generate report
    generate_report
    
    # Ask for cleanup
    read -p "Clean up test files? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cleanup
    fi
    
    echo ""
    echo "✅ Memory leak detection completed"
}

# Handle script interruption
trap 'echo ""; print_error "Script interrupted"; exit 1' INT

# Run main function
main "$@"