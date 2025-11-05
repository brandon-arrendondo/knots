#!/bin/bash
#
# Install knots pre-commit hook
#
# Usage:
#   ./install-hook.sh                    # Install blocking hook
#   ./install-hook.sh --warning-only     # Install warning-only hook
#   ./install-hook.sh --uninstall        # Remove hook
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
MODE="blocking"
HOOK_DIR=".git/hooks"
HOOK_PATH="$HOOK_DIR/pre-commit"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --warning-only)
            MODE="warning"
            shift
            ;;
        --uninstall)
            MODE="uninstall"
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --warning-only    Install warning-only hook (doesn't block commits)"
            echo "  --uninstall       Remove the hook"
            echo "  -h, --help        Show this help"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo -e "${RED}Error: Not in a git repository${NC}"
    echo "Run this script from the root of your git repository"
    exit 1
fi

# Uninstall mode
if [ "$MODE" = "uninstall" ]; then
    if [ -f "$HOOK_PATH" ] && grep -q "knots" "$HOOK_PATH"; then
        rm "$HOOK_PATH"
        echo -e "${GREEN}✓ Pre-commit hook removed${NC}"
    else
        echo -e "${YELLOW}No knots hook found${NC}"
    fi
    exit 0
fi

# Check if knots is installed
if ! command -v knots &> /dev/null; then
    echo -e "${YELLOW}Warning: knots not found in PATH${NC}"
    echo ""
    echo "Install it first:"
    echo "  cargo install knots"
    echo "  # or"
    echo "  cargo build --release"
    echo "  sudo cp target/release/knots /usr/local/bin/"
    echo ""
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOK_DIR"

# Backup existing hook if present
if [ -f "$HOOK_PATH" ]; then
    echo -e "${YELLOW}Existing pre-commit hook found${NC}"
    
    if grep -q "knots" "$HOOK_PATH"; then
        echo "Existing hook is already a knots hook"
        read -p "Overwrite? (y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 0
        fi
    else
        BACKUP="$HOOK_PATH.backup.$(date +%Y%m%d%H%M%S)"
        cp "$HOOK_PATH" "$BACKUP"
        echo -e "${GREEN}Backed up existing hook to: $BACKUP${NC}"
    fi
fi

# Install the appropriate hook
if [ "$MODE" = "warning" ]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    if [ -f "$SCRIPT_DIR/pre-commit-warning-only" ]; then
        cp "$SCRIPT_DIR/pre-commit-warning-only" "$HOOK_PATH"
    else
        echo -e "${RED}Error: pre-commit-warning-only script not found${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ Installed warning-only pre-commit hook${NC}"
else
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    if [ -f "$SCRIPT_DIR/pre-commit" ]; then
        cp "$SCRIPT_DIR/pre-commit" "$HOOK_PATH"
    else
        echo -e "${RED}Error: pre-commit script not found${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ Installed blocking pre-commit hook${NC}"
fi

chmod +x "$HOOK_PATH"

# Configuration suggestions
echo ""
echo -e "${BLUE}Configuration:${NC}"
echo "Set thresholds (default: 15 for both):"
echo "  git config hooks.knots.mccabe-threshold 10"
echo "  git config hooks.knots.cognitive-threshold 15"
echo ""
echo "Enable verbose output:"
echo "  git config hooks.knots.verbose true"
echo ""
echo "Set custom path:"
echo "  git config hooks.knots.path /path/to/knots"
echo ""

# Test the hook
echo -e "${BLUE}Testing hook...${NC}"
if [ "$MODE" = "warning" ]; then
    echo "Hook will show warnings but allow all commits"
else
    echo "Hook will block commits that exceed thresholds"
fi

echo ""
echo -e "${GREEN}Installation complete!${NC}"
