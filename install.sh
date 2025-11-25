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

        # Functional test: verify Ollama actually works
        print_info "Testing Ollama functionality..."
        if ollama list >/dev/null 2>&1; then
            print_success "Ollama is functional"
            return 0
        else
            print_warning "Ollama binary exists but is not functional"
            print_info "Will attempt reinstallation"
            return 1
        fi
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
                print_info "Ollama installer completed"
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
                    print_info "Homebrew installation completed"
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

    # POST-INSTALL VERIFICATION
    print_step "Verifying Ollama installation..."

    # Refresh PATH to pick up newly installed binary
    export PATH="/usr/local/bin:/usr/bin:/bin:$PATH"
    hash -r 2>/dev/null || true

    # Check if binary exists
    if command -v ollama >/dev/null 2>&1; then
        OLLAMA_VERSION=$(ollama --version 2>/dev/null || echo "unknown")
        print_success "Ollama binary verified (${OLLAMA_VERSION})"

        # Check if systemd service was created (Linux only)
        if [ "$OS" = "linux" ] && command_exists systemctl; then
            if systemctl list-unit-files 2>/dev/null | grep -q "ollama.service"; then
                print_success "Ollama systemd service detected"
            else
                print_warning "Ollama systemd service not found (will use manual start)"
            fi
        fi
    else
        print_error "Ollama binary not found after installation"
        print_info "Tried paths: /usr/local/bin/ollama, /usr/bin/ollama"
        print_info "Current PATH: $PATH"
        print_info "Please install manually from: https://ollama.com/download"
        exit 1
    fi

    print_success "Ollama installation verified"
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

    OLLAMA_LOG="/tmp/ollama-install.log"

    case "$OS" in
        linux)
            if command_exists systemctl; then
                # Check if systemd service exists before trying to start
                if systemctl list-unit-files 2>/dev/null | grep -q "ollama.service"; then
                    print_info "Attempting to start Ollama via systemd..."
                    if sudo systemctl start ollama 2>&1 | tee -a "$OLLAMA_LOG"; then
                        print_success "Started Ollama via systemd"
                    else
                        print_warning "Failed to start via systemd (see $OLLAMA_LOG)"
                        print_info "Trying background start..."
                        nohup ollama serve >>"$OLLAMA_LOG" 2>&1 &
                        OLLAMA_PID=$!
                        print_info "Started background process (PID: $OLLAMA_PID, log: $OLLAMA_LOG)"
                    fi
                else
                    print_warning "Ollama systemd service not found"
                    print_info "Starting Ollama in background..."
                    nohup ollama serve >>"$OLLAMA_LOG" 2>&1 &
                    OLLAMA_PID=$!
                    print_info "Started background process (PID: $OLLAMA_PID, log: $OLLAMA_LOG)"
                fi
            else
                print_info "systemctl not available, starting in background..."
                nohup ollama serve >>"$OLLAMA_LOG" 2>&1 &
                OLLAMA_PID=$!
                print_info "Started background process (PID: $OLLAMA_PID, log: $OLLAMA_LOG)"
            fi
            ;;
        darwin)
            print_info "Starting Ollama..."
            if [ -d "/Applications/Ollama.app" ]; then
                open -a Ollama 2>&1 | tee -a "$OLLAMA_LOG" || {
                    print_warning "Failed to open Ollama.app"
                    print_info "Starting in background..."
                    nohup ollama serve >>"$OLLAMA_LOG" 2>&1 &
                    OLLAMA_PID=$!
                    print_info "Started background process (PID: $OLLAMA_PID, log: $OLLAMA_LOG)"
                }
            else
                nohup ollama serve >>"$OLLAMA_LOG" 2>&1 &
                OLLAMA_PID=$!
                print_info "Started background process (PID: $OLLAMA_PID, log: $OLLAMA_LOG)"
            fi
            ;;
    esac

    # Verify background process is running if we started one
    if [ -n "$OLLAMA_PID" ]; then
        sleep 2
        if ps -p $OLLAMA_PID >/dev/null 2>&1; then
            print_success "Ollama process verified running (PID: $OLLAMA_PID)"
        else
            print_error "Ollama process failed to start"
            print_info "Check logs at: $OLLAMA_LOG"
            print_info "Last 10 lines of log:"
            tail -10 "$OLLAMA_LOG" 2>/dev/null || echo "  (log file empty or not found)"
            exit 1
        fi
    fi

    print_info "Waiting for Ollama HTTP API to respond..."
    for i in {1..30}; do
        if check_ollama_running; then
            print_success "Ollama is running and responding"
            return 0
        fi
        sleep 1
        echo -n "."
    done

    echo ""
    print_error "Ollama failed to respond after 30 seconds"
    print_info "Debug information:"
    print_info "  - Check if process is running: ps aux | grep ollama"
    print_info "  - Check logs at: $OLLAMA_LOG"
    print_info "  - Try starting manually: ollama serve"
    print_info "  - Check port 11434: curl http://127.0.0.1:11434/api/tags"
    if [ -f "$OLLAMA_LOG" ]; then
        print_info "Last 10 lines of log:"
        tail -10 "$OLLAMA_LOG"
    fi
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
    print_step "Checking for models..."

    # Check if Ollama is responding first
    if ! check_ollama_running; then
        print_error "Cannot check models: Ollama is not running"
        print_info "Ensure Ollama is running with: ollama serve"
        return 1
    fi

    # Check if any qwen2.5 model exists
    MODEL_LIST=$(ollama list 2>&1)
    if echo "$MODEL_LIST" | grep -q "qwen2.5"; then
        EXISTING_MODEL=$(echo "$MODEL_LIST" | grep "qwen2.5" | head -1 | awk '{print $1}')
        print_success "Found existing model: ${EXISTING_MODEL}"
        print_info "Will use ${EXISTING_MODEL} as default"
        return 0
    elif echo "$MODEL_LIST" | grep -q "$DEFAULT_MODEL"; then
        print_success "Model ${DEFAULT_MODEL} is already installed"
        return 0
    else
        print_warning "No qwen2.5 models installed"
        print_info "Available models:"
        echo "$MODEL_LIST" | head -10
        return 1
    fi
}

pull_model() {
    print_step "Downloading model (${DEFAULT_MODEL})..."
    print_info "This may take a few minutes depending on your connection..."
    print_info "Model size: approximately 3.8 GB"

    # Check available disk space (rough check)
    AVAILABLE_SPACE_KB=$(df -k "$HOME" 2>/dev/null | awk 'NR==2 {print $4}')
    if [ -n "$AVAILABLE_SPACE_KB" ] && [ "$AVAILABLE_SPACE_KB" -lt 4000000 ]; then
        print_warning "Low disk space detected (less than 4 GB available)"
        print_info "Model download requires approximately 3.8 GB"
        print_info "Available space: $(($AVAILABLE_SPACE_KB / 1024 / 1024)) GB"
    fi

    # Check if Ollama is responding
    if ! check_ollama_running; then
        print_error "Cannot pull model: Ollama is not running"
        print_info "Start Ollama with: ollama serve"
        print_info "Then pull model with: ollama pull ${DEFAULT_MODEL}"
        return 1
    fi

    if ollama pull "$DEFAULT_MODEL" 2>&1; then
        print_success "Model downloaded successfully"

        # Verify model is actually available
        if ollama list 2>/dev/null | grep -q "$DEFAULT_MODEL"; then
            print_success "Model verified in local repository"
        else
            print_warning "Model pull completed but not found in list"
            print_info "Run 'ollama list' to check available models"
        fi
    else
        print_error "Failed to download model"
        print_info "Common issues:"
        print_info "  - Network connectivity problems"
        print_info "  - Insufficient disk space"
        print_info "  - Ollama service not responding"
        print_info "You can download it later with: ollama pull ${DEFAULT_MODEL}"
        print_info "Check Ollama logs at: /tmp/ollama-install.log"
        return 1
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
    # Auto-download model when running via pipe
    if [ -t 0 ]; then
        printf "Download default model ${DEFAULT_MODEL} (3.8GB)? [Y/n] "
        read -r REPLY
    else
        # Running via pipe, auto-download
        echo "Auto-downloading model ${DEFAULT_MODEL} (running via pipe)..."
        REPLY="Y"
    fi
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
