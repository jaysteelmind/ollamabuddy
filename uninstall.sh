#!/bin/bash
set -e

#############################################################
# OllamaBuddy Uninstaller
# Usage: curl -fsSL https://raw.githubusercontent.com/jaysteelmind/ollamabuddy/main/uninstall.sh | sh
#
# This script will:
# 1. Remove OllamaBuddy binary
# 2. Optionally remove configuration and state
# 3. Optionally remove Ollama
# 4. Optionally remove downloaded models
#############################################################

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="${HOME}/.ollamabuddy"
BINARY_PATH="${INSTALL_DIR}/ollamabuddy"

#############################################################
# Helper Functions
#############################################################

print_banner() {
    echo -e "${CYAN}"
    cat << 'EOF'
   ___  _ _                       ____            _     _       
  / _ \| | | __ _ _ __ ___   __ _| __ ) _   _  __| | __| |_   _ 
 | | | | | |/ _` | '_ ` _ \ / _` |  _ \| | | |/ _` |/ _` | | | |
 | |_| | | | (_| | | | | | | (_| | |_) | |_| | (_| | (_| | |_| |
  \___/|_|_|\__,_|_| |_| |_|\__,_|____/ \__,_|\__,_|\__,_|\__, |
                                                           |___/ 
                            Uninstaller
EOF
    echo -e "${NC}\n"
}

print_success() { echo -e "${GREEN}[OK] $1${NC}"; }
print_error() { echo -e "${RED}[ERROR] $1${NC}"; }
print_info() { echo -e "${BLUE}[INFO] $1${NC}"; }
print_warning() { echo -e "${YELLOW}[WARN] $1${NC}"; }
print_step() { echo -e "\n${MAGENTA}==> ${BLUE}$1${NC}"; }

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

confirm() {
    local prompt="$1"
    local default="${2:-N}"
    
    if [ "$default" = "Y" ]; then
        read -p "$prompt [Y/n] " -n 1 -r
    else
        read -p "$prompt [y/N] " -n 1 -r
    fi
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]] || ( [[ -z $REPLY ]] && [[ $default = "Y" ]] ); then
        return 0
    else
        return 1
    fi
}

#############################################################
# Removal Functions
#############################################################

remove_binary() {
    print_step "Removing OllamaBuddy binary..."
    
    if [ ! -f "$BINARY_PATH" ]; then
        print_warning "OllamaBuddy binary not found at ${BINARY_PATH}"
        return 0
    fi
    
    if [ -w "$INSTALL_DIR" ]; then
        rm -f "$BINARY_PATH"
        print_success "Removed ${BINARY_PATH}"
    else
        print_info "Need sudo to remove from ${INSTALL_DIR}"
        if sudo rm -f "$BINARY_PATH"; then
            print_success "Removed ${BINARY_PATH}"
        else
            print_error "Failed to remove binary"
            return 1
        fi
    fi
}

remove_config() {
    print_step "Removing configuration and state..."
    
    if [ ! -d "$CONFIG_DIR" ]; then
        print_info "No configuration directory found at ${CONFIG_DIR}"
        return 0
    fi
    
    if confirm "Remove configuration and state (${CONFIG_DIR})?" "N"; then
        if rm -rf "$CONFIG_DIR"; then
            print_success "Removed ${CONFIG_DIR}"
        else
            print_error "Failed to remove configuration directory"
            return 1
        fi
    else
        print_info "Keeping configuration at ${CONFIG_DIR}"
    fi
}

remove_ollama() {
    print_step "Checking Ollama installation..."
    
    if ! command_exists ollama; then
        print_info "Ollama is not installed"
        return 0
    fi
    
    echo ""
    print_warning "Ollama may be used by other applications"
    
    if confirm "Remove Ollama?" "N"; then
        OS=$(uname -s | tr '[:upper:]' '[:lower:]')
        
        case "$OS" in
            linux*)
                print_info "Removing Ollama on Linux..."
                
                # Stop service
                if command_exists systemctl; then
                    sudo systemctl stop ollama 2>/dev/null || true
                    sudo systemctl disable ollama 2>/dev/null || true
                fi
                
                # Remove binary
                sudo rm -f /usr/local/bin/ollama
                sudo rm -f /usr/bin/ollama
                
                # Remove service file
                sudo rm -f /etc/systemd/system/ollama.service
                
                # Reload systemd if needed
                if command_exists systemctl; then
                    sudo systemctl daemon-reload 2>/dev/null || true
                fi
                
                print_success "Ollama removed"
                ;;
                
            darwin*)
                print_info "Removing Ollama on macOS..."
                
                if command_exists brew && brew list ollama &>/dev/null; then
                    brew uninstall ollama
                    print_success "Ollama removed via Homebrew"
                else
                    # Remove app if installed
                    if [ -d "/Applications/Ollama.app" ]; then
                        rm -rf "/Applications/Ollama.app"
                    fi
                    
                    # Remove CLI if installed
                    sudo rm -f /usr/local/bin/ollama
                    
                    print_success "Ollama removed"
                fi
                ;;
                
            *)
                print_warning "Manual Ollama removal required for this OS"
                ;;
        esac
    else
        print_info "Keeping Ollama installed"
    fi
}

remove_models() {
    if ! command_exists ollama; then
        print_info "Ollama not installed, skipping model removal"
        return 0
    fi
    
    print_step "Checking Ollama models..."
    
    MODELS=$(ollama list 2>/dev/null | tail -n +2 | awk '{print $1}' || echo "")
    
    if [ -z "$MODELS" ]; then
        print_info "No Ollama models installed"
        return 0
    fi
    
    echo ""
    print_info "Installed models:"
    echo "$MODELS" | while read -r model; do
        echo "  - $model"
    done
    echo ""
    
    if confirm "Remove all Ollama models?" "N"; then
        echo "$MODELS" | while read -r model; do
            if [ -n "$model" ]; then
                print_info "Removing model: $model"
                ollama rm "$model" 2>/dev/null || print_warning "Failed to remove $model"
            fi
        done
        print_success "Models removed"
    else
        print_info "Keeping Ollama models"
    fi
}

#############################################################
# Summary
#############################################################

print_summary() {
    echo ""
    echo "================================================================"
    echo ""
    echo "  OllamaBuddy Uninstallation Complete!"
    echo ""
    echo "================================================================"
    echo ""
    print_success "OllamaBuddy has been removed from your system"
    echo ""
    
    if [ -d "$CONFIG_DIR" ]; then
        echo "Remaining files:"
        echo "  - Configuration: ${CONFIG_DIR}"
        echo "  - Remove manually with: rm -rf ${CONFIG_DIR}"
        echo ""
    fi
    
    if command_exists ollama; then
        echo "Ollama is still installed:"
        echo "  - Run this uninstaller again to remove Ollama"
        echo "  - Or remove manually (Linux): sudo rm /usr/local/bin/ollama"
        echo "  - Or remove manually (macOS): brew uninstall ollama"
        echo ""
    fi
    
    echo "================================================================"
    echo ""
}

#############################################################
# Main Uninstallation Flow
#############################################################

main() {
    print_banner
    
    echo "This will remove OllamaBuddy from your system."
    echo ""
    
    if ! confirm "Continue with uninstallation?" "Y"; then
        print_info "Uninstallation cancelled"
        exit 0
    fi
    
    remove_binary
    
    remove_config
    
    echo ""
    print_info "OllamaBuddy binary has been removed"
    print_info "You can also remove Ollama and models if no longer needed"
    echo ""
    
    if confirm "Remove Ollama as well?" "N"; then
        remove_models
        remove_ollama
    fi
    
    print_summary
}

main "$@"
