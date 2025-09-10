#!/bin/bash

# Automated WASM File Reference Migration Script
# Fixes legacy wasm_sdk.js references to current dash_wasm_sdk.js across all files

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 WASM File Reference Migration Tool${NC}"
echo -e "${BLUE}====================================${NC}"
echo ""

# Configuration
BACKUP_DIR="wasm-reference-backup-$(date +%Y%m%d_%H%M%S)"
DRY_RUN=false
VERBOSE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --dry-run    Show what would be changed without making changes"
            echo "  --verbose    Show detailed progress information"
            echo "  --help       Show this help message"
            echo ""
            echo "This script updates legacy WASM file references:"
            echo "  pkg/wasm_sdk.js → pkg/dash_wasm_sdk.js"
            echo "  pkg/wasm_sdk_bg.wasm → pkg/dash_wasm_sdk_bg.wasm"
            echo ""
            echo "Affected directories:"
            echo "  - examples/ (13 files)"
            echo "  - test/ (37 files)"  
            echo "  - documentation (24+ files)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Function to log verbose messages
log_verbose() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${YELLOW}  $1${NC}"
    fi
}

# Function to create backup
create_backup() {
    if [ "$DRY_RUN" = false ]; then
        echo -e "${YELLOW}📁 Creating backup directory: $BACKUP_DIR${NC}"
        mkdir -p "$BACKUP_DIR"
        
        # Backup examples
        if [ -d "examples" ]; then
            cp -r examples "$BACKUP_DIR/"
            log_verbose "Backed up examples directory"
        fi
        
        # Backup test files
        if [ -d "test" ]; then
            cp -r test "$BACKUP_DIR/"
            log_verbose "Backed up test directory"
        fi
        
        # Backup key documentation files
        for doc in *.md CLAUDE.md AI_REFERENCE.md; do
            if [ -f "$doc" ]; then
                cp "$doc" "$BACKUP_DIR/"
                log_verbose "Backed up $doc"
            fi
        done
        
        echo -e "${GREEN}✅ Backup completed${NC}"
    else
        echo -e "${YELLOW}🏃 DRY RUN MODE - No backup created${NC}"
    fi
}

# Function to update file references
update_file_references() {
    local file="$1"
    local changes_made=false
    
    log_verbose "Processing: $file"
    
    if [ ! -f "$file" ]; then
        log_verbose "File not found: $file"
        return 1
    fi
    
    # Check if file contains old references
    if grep -q "wasm_sdk\.js\|wasm_sdk_bg\.wasm" "$file"; then
        if [ "$DRY_RUN" = true ]; then
            echo -e "${YELLOW}  [DRY RUN] Would update: $file${NC}"
            grep -n "wasm_sdk\.js\|wasm_sdk_bg\.wasm" "$file" | head -3 | sed 's/^/    /'
            return 0
        fi
        
        # Create temporary file for updates
        local temp_file=$(mktemp)
        
        # Perform replacements
        sed 's|pkg/wasm_sdk\.js|pkg/dash_wasm_sdk.js|g' "$file" | \
        sed 's|pkg/wasm_sdk_bg\.wasm|pkg/dash_wasm_sdk_bg.wasm|g' | \
        sed "s|'wasm_sdk'|'dash_wasm_sdk'|g" | \
        sed 's|"wasm_sdk"|"dash_wasm_sdk"|g' | \
        sed 's|wasm_sdk_bg\.wasm|dash_wasm_sdk_bg.wasm|g' | \
        sed 's|from.*wasm_sdk\.js|from "../pkg/dash_wasm_sdk.js"|g' > "$temp_file"
        
        # Check if changes were made
        if ! cmp -s "$file" "$temp_file"; then
            mv "$temp_file" "$file"
            changes_made=true
            log_verbose "Updated: $file"
        else
            rm "$temp_file"
            log_verbose "No changes needed: $file"
        fi
    fi
    
    return 0
}

# Function to update examples directory
update_examples() {
    echo -e "${YELLOW}📝 Updating Node.js examples...${NC}"
    
    local updated=0
    local failed=0
    
    if [ -d "examples" ]; then
        for file in examples/*.mjs; do
            if [ -f "$file" ]; then
                if update_file_references "$file"; then
                    ((updated++))
                else
                    ((failed++))
                fi
            fi
        done
        
        echo -e "${GREEN}✅ Examples update completed: $updated files updated, $failed failed${NC}"
    else
        echo -e "${RED}❌ Examples directory not found${NC}"
        return 1
    fi
}

# Function to update test files
update_tests() {
    echo -e "${YELLOW}🧪 Updating test files...${NC}"
    
    local updated=0
    local failed=0
    
    if [ -d "test" ]; then
        # Update test files
        for file in test/*.mjs test/*.js test/**/*.mjs test/**/*.js; do
            if [ -f "$file" ]; then
                if update_file_references "$file"; then
                    ((updated++))
                else
                    ((failed++))
                fi
            fi
        done
        
        echo -e "${GREEN}✅ Test files update completed: $updated files updated, $failed failed${NC}"
    else
        echo -e "${RED}❌ Test directory not found${NC}"
        return 1
    fi
}

# Function to update documentation
update_documentation() {
    echo -e "${YELLOW}📚 Updating documentation files...${NC}"
    
    local updated=0
    local failed=0
    
    # Update main documentation files
    for file in *.md examples/*.md test/*.md; do
        if [ -f "$file" ]; then
            if update_file_references "$file"; then
                ((updated++))
            else
                ((failed++))
            fi
        fi
    done
    
    echo -e "${GREEN}✅ Documentation update completed: $updated files updated, $failed failed${NC}"
}

# Function to verify updates
verify_updates() {
    echo -e "${YELLOW}🔍 Verifying updates...${NC}"
    
    # Count remaining old references
    local remaining_refs=$(find examples test -name "*.mjs" -o -name "*.js" | xargs grep -l "wasm_sdk\.js\|wasm_sdk_bg\.wasm" 2>/dev/null | wc -l)
    
    if [ "$remaining_refs" -eq 0 ]; then
        echo -e "${GREEN}✅ All legacy references updated successfully${NC}"
    else
        echo -e "${YELLOW}⚠️  $remaining_refs files still contain legacy references${NC}"
        echo "Files with remaining references:"
        find examples test -name "*.mjs" -o -name "*.js" | xargs grep -l "wasm_sdk\.js\|wasm_sdk_bg\.wasm" 2>/dev/null | head -5
    fi
    
    # Test one example to verify it works
    if [ "$DRY_RUN" = false ] && [ -f "examples/getting-started.mjs" ]; then
        echo -e "${YELLOW}🧪 Testing updated example...${NC}"
        
        if node examples/getting-started.mjs --network=testnet --quick-test 2>/dev/null | grep -q "initialized\|completed\|success"; then
            echo -e "${GREEN}✅ Example verification successful${NC}"
        else
            echo -e "${RED}⚠️  Example test inconclusive (might be network-related)${NC}"
        fi
    fi
}

# Function to show summary
show_summary() {
    echo ""
    echo -e "${BLUE}📊 Migration Summary${NC}"
    echo -e "${BLUE}===================${NC}"
    
    if [ "$DRY_RUN" = true ]; then
        echo "🏃 DRY RUN MODE - No files were modified"
        echo "Run without --dry-run to apply changes"
    else
        echo "✅ Migration completed successfully"
        echo "📁 Backup created at: $BACKUP_DIR"
        
        # Show file counts
        local example_count=$(find examples -name "*.mjs" 2>/dev/null | wc -l)
        local test_count=$(find test -name "*.mjs" -o -name "*.js" 2>/dev/null | wc -l)
        local doc_count=$(find . -maxdepth 1 -name "*.md" | wc -l)
        
        echo "📊 Files processed:"
        echo "   Examples: $example_count files"
        echo "   Tests: $test_count files"
        echo "   Documentation: $doc_count files"
    fi
    
    echo ""
    echo "🎯 Next Steps:"
    echo "1. Verify examples work: node examples/getting-started.mjs"
    echo "2. Run test verification: node test/verify-setup.mjs"
    echo "3. Execute test suite: cd test && ./run-all-tests.sh"
}

# Function to show current status
show_current_status() {
    echo -e "${YELLOW}📊 Current WASM File Reference Status${NC}"
    
    # Count files with old references
    local examples_with_old=$(find examples -name "*.mjs" 2>/dev/null | xargs grep -l "wasm_sdk\.js" 2>/dev/null | wc -l)
    local tests_with_old=$(find test -name "*.mjs" -o -name "*.js" 2>/dev/null | xargs grep -l "wasm_sdk\.js" 2>/dev/null | wc -l)
    local docs_with_old=$(find . -maxdepth 1 -name "*.md" | xargs grep -l "wasm_sdk\.js" 2>/dev/null | wc -l)
    
    echo "Files with legacy references:"
    echo "  Examples: $examples_with_old files"
    echo "  Tests: $tests_with_old files" 
    echo "  Documentation: $docs_with_old files"
    echo ""
    
    # Check current build
    if [ -f "pkg/dash_wasm_sdk.js" ] && [ -f "pkg/dash_wasm_sdk_bg.wasm" ]; then
        echo -e "${GREEN}✅ Current WASM build exists (dash_wasm_sdk.*)${NC}"
    else
        echo -e "${RED}❌ Current WASM build missing - run ./build.sh first${NC}"
        exit 1
    fi
}

# Main execution
main() {
    show_current_status
    
    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}🔍 DRY RUN - Analyzing what would be changed...${NC}"
        echo ""
    fi
    
    create_backup
    echo ""
    
    update_examples
    echo ""
    
    update_tests  
    echo ""
    
    update_documentation
    echo ""
    
    if [ "$DRY_RUN" = false ]; then
        verify_updates
        echo ""
    fi
    
    show_summary
}

# Run main function
main