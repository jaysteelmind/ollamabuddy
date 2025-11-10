#!/bin/bash
set -e

#############################################################
# OllamaBuddy All-in-One Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/jaysteelmind/ollamabuddy/master/install.sh | sh
#
# This script will:
# 1. Detect your OS and architecture
# 2. Check/install Ollama
# 3. Download and install OllamaBuddy
# 4. Pull the default model
# 5. Run diagnostics to verify installation
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
OLLAMABUDDY_VERSION="${OLLAMABUDDY_VERSION:-latest}"
DEFAULT_MODEL="qwen2.5:7b-instruct"
INSTALL_DIR="/usr/local/bin"
GITHUB_REPO="jaysteelmind/ollamabuddy"
CONFIG_DIR="${HOME}/.ollamabuddy"

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
         Turn any local Ollama model into a capable terminal agent
                        v0.2.0 - All-in-One Installer
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

#############################################################
# Platform Detection
#############################################################

detect_platform() {
    print_step "Detecting platform..."
    
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case "$OS" in
        linux*)
            OS="linux"
            ;;
        darwin*)
            OS="darwin"
            ;;
        *)
            print_error "Unsupported operating system: $OS"
            print_info "OllamaBuddy currently supports Linux and macOS"
            exit 1
            ;;
    esac
    
    case "$ARCH" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            print_error "Unsupported architecture: $ARCH"
            print_info "OllamaBuddy currently supports x86_64 and aarch64"
            exit 1
            ;;
    esac
    
    print_success "Platform: ${OS}-${ARCH}"
}

#############################################################
# Dependency Checks
#############################################################

check_dependencies() {
    print_step "Checking dependencies..."
    
    MISSING_DEPS=""
    
    if ! command_exists curl; then
        MISSING_DEPS="curl"
    fi
    
    if [ -n "$MISSING_DEPS" ]; then
        print_error "Missing required dependencies: $MISSING_DEPS"
        print_info "Please install them and try again"
        exit 1
    fi
    
    print_success "All dependencies present"
}

#############################################################
# Ollama Installation
#############################################################

check_ollama() {
    print_step "Checking for Ollama..."
    
    if command_exists ollama; then
        OLLAMA_VERSION=$(ollama --version 2>/dev/null || echo "unknown")
        print_success "Ollama is installed (${OLLAMA_VERSION})"
        return 0
    else
        print_warning "Ollama is not installed"
        return 1
    fi
}

install_ollama() {
    print_step "Installing Ollama..."
    
    case "$OS" in
        linux)
            print_info "Installing Ollama for Linux..."
            if curl -fsSL https://ollama.com/install.sh | sh; then
                print_success "Ollama installed successfully"
            else
                print_error "Failed to install Ollama"
                print_info "Please install manually from: https://ollama.com/download"
                exit 1
            fi
            ;;
        darwin)
            print_info "Installing Ollama for macOS..."
            if command_exists brew; then
                if brew install ollama; then
                    print_success "Ollama installed via Homebrew"
                else
                    print_error "Failed to install Ollama via Homebrew"
                    exit 1
                fi
            else
                print_warning "Homebrew not found"
                print_info "Please install Ollama manually from: https://ollama.com/download"
                print_info "Or install Homebrew first: https://brew.sh"
                exit 1
            fi
            ;;
    esac
}

check_ollama_running() {
    if curl -s --max-time 2 http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

start_ollama() {
    print_step "Starting Ollama service..."
    
    case "$OS" in
        linux)
            if command_exists systemctl; then
                if sudo systemctl start ollama 2>/dev/null; then
                    print_info "Started Ollama via systemd"
                else
                    print_info "Starting Ollama in background..."
                    nohup ollama serve >/dev/null 2>&1 &
                fi
            else
                print_info "Starting Ollama in background..."
                nohup ollama serve >/dev/null 2>&1 &
            fi
            ;;
        darwin)
            print_info "Starting Ollama..."
            if [ -d "/Applications/Ollama.app" ]; then
                open -a Ollama 2>/dev/null || nohup ollama serve >/dev/null 2>&1 &
            else
                nohup ollama serve >/dev/null 2>&1 &
            fi
            ;;
    esac
    
    print_info "Waiting for Ollama to start..."
    for i in {1..30}; do
        if check_ollama_running; then
            print_success "Ollama is running"
            return 0
        fi
        sleep 1
        echo -n "."
    done
    
    echo ""
    print_error "Failed to start Ollama automatically"
    print_info "Please start Ollama manually with: ollama serve"
    print_info "Then run this installer again"
    exit 1
}

ensure_ollama_running() {
    print_step "Ensuring Ollama is running..."
    
    if check_ollama_running; then
        print_success "Ollama is already running"
    else
        start_ollama
    fi
}

#############################################################
# OllamaBuddy Installation
#############################################################

build_from_source() {
    print_step "Building OllamaBuddy from source..."
    
    if ! command_exists cargo; then
        print_warning "Rust is not installed"
        print_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        export PATH="$HOME/.cargo/bin:$PATH"
        source "$HOME/.cargo/env" 2>/dev/null || true
    fi
    
    print_success "Rust is installed"
    
    TMP_DIR=$(mktemp -d)
    cd "$TMP_DIR"
    
    print_info "Cloning repository..."
    if command_exists git; then
        git clone https://github.com/${GITHUB_REPO}.git ollamabuddy
    else
        print_info "git not found, downloading as archive..."
        curl -fsSL "https://github.com/${GITHUB_REPO}/archive/refs/heads/master.tar.gz" -o ollamabuddy.tar.gz
        tar -xzf ollamabuddy.tar.gz
        mv ollamabuddy-master ollamabuddy
    fi
    
    cd ollamabuddy
    
    print_info "Building OllamaBuddy (this may take a few minutes)..."
    if cargo build --release; then
        print_success "Build completed successfully"
        
        install_binary "target/release/ollamabuddy"
        
        cd /
        rm -rf "$TMP_DIR"
    else
        print_error "Build failed"
        cd /
        rm -rf "$TMP_DIR"
        exit 1
    fi
}

install_binary() {
    local BINARY_PATH="$1"
    
    print_step "Installing OllamaBuddy to ${INSTALL_DIR}..."
    
    if [ -w "$INSTALL_DIR" ]; then
        cp "$BINARY_PATH" "${INSTALL_DIR}/ollamabuddy"
        chmod +x "${INSTALL_DIR}/ollamabuddy"
    else
        print_info "Need sudo to install to ${INSTALL_DIR}"
        sudo cp "$BINARY_PATH" "${INSTALL_DIR}/ollamabuddy"
        sudo chmod +x "${INSTALL_DIR}/ollamabuddy"
    fi
    
    print_success "OllamaBuddy installed to ${INSTALL_DIR}/ollamabuddy"
}

#############################################################
# Model Installation
#############################################################

check_model() {
    print_step "Checking for default model (${DEFAULT_MODEL})..."
    
    if ollama list 2>/dev/null | grep -q "$DEFAULT_MODEL"; then
        print_success "Model ${DEFAULT_MODEL} is already installed"
        return 0
    else
        print_warning "Model ${DEFAULT_MODEL} is not installed"
        return 1
    fi
}

pull_model() {
    print_step "Downloading model (${DEFAULT_MODEL})..."
    print_info "This may take a few minutes depending on your connection..."
    print_info "Model size: approximately 3.8 GB"
    
    if ollama pull "$DEFAULT_MODEL"; then
        print_success "Model downloaded successfully"
    else
        print_error "Failed to download model"
        print_info "You can download it later with: ollama pull ${DEFAULT_MODEL}"
    fi
}

#############################################################
# Configuration
#############################################################

setup_config() {
    print_step "Setting up configuration..."
    
    mkdir -p "$CONFIG_DIR"
    
    if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
        cat > "${CONFIG_DIR}/config.toml" << 'CONFIGEOF'
[ollama]
host = "127.0.0.1"
port = 11434
default_model = "qwen2.5:7b-instruct"

[agent]
max_context_tokens = 8000
compress_threshold = 6000
max_memory_entries = 100
max_iterations = 10
timeout_minutes = 30

[tools]
default_timeout_sec = 30
max_output_bytes = 2000000
online_enabled = false
max_parallel = 4

[advisor]
auto_upgrade = false
cost_sensitivity = 0.3
upgrade_threshold = 0.15

[telemetry]
default_verbosity = "normal"
show_progress_bars = true
color_output = true

[paths]
state_dir = "~/.ollamabuddy"
log_dir = "~/.ollamabuddy/logs"
CONFIGEOF
        print_success "Created default configuration at ${CONFIG_DIR}/config.toml"
    else
        print_info "Configuration already exists at ${CONFIG_DIR}/config.toml"
    fi
}

#############################################################
# Verification
#############################################################

run_diagnostics() {
    print_step "Running system diagnostics..."
    
    if ollamabuddy doctor; then
        print_success "All system checks passed"
    else
        print_warning "Some system checks failed"
        print_info "Run 'ollamabuddy doctor' for details"
    fi
}

#############################################################
# Installation Summary
#############################################################

print_summary() {
    echo ""
    echo "================================================================"
    echo ""
    echo "  OllamaBuddy Installation Complete!"
    echo ""
    echo "================================================================"
    echo ""
    echo "Installed Components:"
    echo "  - OllamaBuddy v0.2.0"
    echo "  - Ollama (local LLM server)"
    echo "  - Model: ${DEFAULT_MODEL}"
    echo ""
    echo "Quick Start:"
    echo "  ollamabuddy doctor          - Check system health"
    echo "  ollamabuddy models          - List available models"
    echo "  ollamabuddy config          - Show configuration"
    echo "  ollamabuddy \"your task\"    - Run agent (coming in v0.2.1)"
    echo ""
    echo "Documentation:"
    echo "  https://github.com/${GITHUB_REPO}"
    echo ""
    echo "Examples:"
    echo "  ollamabuddy \"summarize all markdown files\""
    echo "  ollamabuddy -v \"create a Python script\""
    echo "  ollamabuddy --online \"research latest Rust features\""
    echo ""
    echo "================================================================"
    echo ""
}

#############################################################
# Main Installation Flow
#############################################################

main() {
    print_banner
    
    detect_platform
    check_dependencies
    
    if ! check_ollama; then
        install_ollama
    fi
    ensure_ollama_running
    
    # Build from source (no releases yet)
    build_from_source
    
    if ! check_model; then
        echo ""
        printf "Download default model ${DEFAULT_MODEL} (3.8GB)? [Y/n] "
        read REPLY
        if [ "$REPLY" = "Y" ] || [ "$REPLY" = "y" ] || [ -z "$REPLY" ]; then
            pull_model
        else
            print_info "Skipping model download"
            print_info "Download later with: ollama pull ${DEFAULT_MODEL}"
        fi
    fi
    
    setup_config
    
    run_diagnostics
    
    print_summary
}

main "$@"
