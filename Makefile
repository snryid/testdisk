# Mini TestDisk — cross-platform GUI build (Rust + Tauri 1/2)
#
# Usage:
#   make help          Show all targets
#   make dev           Run development GUI (auto-detects Tauri 1 on Ubuntu 20.04)
#   make build         Build release bundle for current OS
#   make build-macos   Build on macOS (.app / .dmg)
#   make build-linux   Build on Linux (.deb / .AppImage)
#   make build-windows Build on Windows (.exe / .msi)

SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

PROJECT_ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
CARGO        ?= cargo
NPM          ?= npm
RUN_TAURI    := $(PROJECT_ROOT)/scripts/run-tauri.sh
TAURI_MAJOR  := $(shell "$(PROJECT_ROOT)/scripts/detect-tauri-version.sh" 2>/dev/null || echo 2)

# Detect host OS (Darwin / Linux / MINGW* / MSYS* / CYGWIN*)
UNAME_S := $(shell uname -s 2>/dev/null || echo Unknown)

ifeq ($(UNAME_S),Darwin)
  HOST_OS := macos
  BUNDLE_DIR := $(PROJECT_ROOT)/target/release/bundle/macos
  BUNDLE_GLOB := *.app
else ifeq ($(UNAME_S),Linux)
  HOST_OS := linux
  BUNDLE_DIR := $(PROJECT_ROOT)/target/release/bundle
  BUNDLE_GLOB := *
else ifneq ($(filter MINGW% MSYS% CYGWIN%,$(UNAME_S)),)
  HOST_OS := windows
  BUNDLE_DIR := $(PROJECT_ROOT)/target/release/bundle/nsis
  BUNDLE_GLOB := *.exe
else
  HOST_OS := unknown
  BUNDLE_DIR := $(PROJECT_ROOT)/target/release/bundle
  BUNDLE_GLOB := *
endif

RUST_VERSION := $(shell rustc --version 2>/dev/null || echo "not installed")
TAURI_VERSION := $(shell $(CARGO) tauri --version 2>/dev/null || echo "not installed")
NODE_VERSION := $(shell node --version 2>/dev/null || echo "not installed")
NPM_VERSION := $(shell $(NPM) --version 2>/dev/null || echo "not installed")

.PHONY: help check check-tauri install-tauri-cli install-tauri-cli-v2 dev dev-v1 dev-v2 dev-xvfb build build-debug \
        build-macos build-linux build-windows test test-core sample-image clean info tauri-version

help: ## Show available targets
	@echo "Mini TestDisk — Makefile targets"
	@echo ""
	@echo "Host OS detected: $(HOST_OS) ($(UNAME_S))"
	@echo "Tauri major (auto): $(TAURI_MAJOR)"
	@echo ""
	@grep -E '^[a-zA-Z0-9_.-]+:.*##' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

tauri-version: ## Print auto-detected Tauri major version (1 or 2)
	@"$(PROJECT_ROOT)/scripts/detect-tauri-version.sh"

check: check-tauri ## Verify Rust, Node, and Tauri CLI for detected major version
	@echo "Rust:        $(RUST_VERSION)"
	@echo "Tauri CLI:   $(TAURI_VERSION)"
	@echo "Tauri major: $(TAURI_MAJOR)"
	@echo "Node:        $(NODE_VERSION)"
	@echo "npm:         $(NPM_VERSION)"
	@command -v $(CARGO) >/dev/null 2>&1 || { echo "Error: cargo not found. Install Rust: https://rustup.rs"; exit 1; }
	@command -v node >/dev/null 2>&1 || { echo "Error: node not found. Install Node.js: https://nodejs.org"; exit 1; }
	@command -v $(NPM) >/dev/null 2>&1 || { echo "Error: npm not found. Install Node.js: https://nodejs.org"; exit 1; }
	@echo "OK — toolchain ready"

check-tauri: ## Verify Tauri CLI for auto-detected major version
ifeq ($(TAURI_MAJOR),1)
	@command -v node >/dev/null 2>&1 || { echo "Error: node required for Tauri 1 CLI (npx @tauri-apps/cli@1)"; exit 1; }
else
	@$(CARGO) tauri --version >/dev/null 2>&1 || { \
		echo "Error: cargo tauri not found. Run: make install-tauri-cli"; \
		exit 1; \
	}
endif

install-tauri-cli: install-tauri-cli-v2 ## Install Tauri 2 CLI (cargo install tauri-cli)

install-tauri-cli-v2: ## Install Tauri 2 CLI only
	$(CARGO) install tauri-cli --locked

dev: check ## Start GUI (auto-detect Tauri 1 on Ubuntu 20.04, else Tauri 2)
	"$(RUN_TAURI)" dev

dev-xvfb: check ## Start GUI on virtual display (SSH/headless smoke test; needs xvfb)
	USE_XVFB=1 "$(RUN_TAURI)" dev

dev-v1: ## Force Tauri 1.x development mode
	TAURI_MAJOR=1 "$(RUN_TAURI)" dev

dev-v2: check-tauri ## Force Tauri 2.x development mode
	TAURI_MAJOR=2 "$(RUN_TAURI)" dev

build: check ## Build release GUI bundle (auto-detect Tauri major)
	"$(RUN_TAURI)" build
	@$(MAKE) --no-print-directory info

build-debug: check ## Build debug GUI bundle for current platform
	"$(RUN_TAURI)" build --debug

build-macos: ## Build macOS .app and .dmg (must run on macOS)
ifneq ($(HOST_OS),macos)
	@echo "Error: build-macos requires macOS (current: $(UNAME_S))"
	@exit 1
endif
	TAURI_MAJOR=2 "$(RUN_TAURI)" build --bundles app,dmg
	@$(MAKE) --no-print-directory info

build-linux: ## Build Linux .deb and AppImage (auto-detect Tauri major)
ifneq ($(HOST_OS),linux)
	@echo "Error: build-linux requires Linux (current: $(UNAME_S))"
	@exit 1
endif
	"$(RUN_TAURI)" build --bundles deb,appimage
	@$(MAKE) --no-print-directory info

build-windows: ## Build Windows .exe and .msi (must run on Windows)
ifneq ($(HOST_OS),windows)
	@echo "Error: build-windows requires Windows (current: $(UNAME_S))"
	@exit 1
endif
	TAURI_MAJOR=2 "$(RUN_TAURI)" build --bundles msi,nsis
	@$(MAKE) --no-print-directory info

test: ## Run Rust unit tests (core crates; Tauri shell if buildable)
	cd "$(PROJECT_ROOT)" && $(CARGO) test -p testdisk-core -p testdisk-platform
ifeq ($(TAURI_MAJOR),1)
	-cd "$(PROJECT_ROOT)/src-tauri-v1" && $(CARGO) test
else
	-cd "$(PROJECT_ROOT)/src-tauri" && $(CARGO) test
endif

test-core: ## Run testdisk-core library tests only
	cd "$(PROJECT_ROOT)" && $(CARGO) test -p testdisk-core

sample-image: ## Generate testdata/sample-mbr.img for offline testing
	@mkdir -p "$(PROJECT_ROOT)/testdata"
	@python3 -c ' \
import struct; \
img = bytearray(4 * 1024 * 1024); \
img[510:512] = b"\x55\xAA"; \
off = 446; \
img[off+4] = 0x0c; \
struct.pack_into("<I", img, off+8, 2048); \
struct.pack_into("<I", img, off+12, 2048); \
fat = 2048 * 512; \
img[fat+3:fat+11] = b"MSWIN4.1"; \
struct.pack_into("<H", img, fat+11, 512); \
img[fat+13] = 8; \
img[fat+21] = 0xF8; \
img[fat+510:fat+512] = b"\x55\xAA"; \
open("$(PROJECT_ROOT)/testdata/sample-mbr.img", "wb").write(img); \
print("Created testdata/sample-mbr.img")'

clean: ## Remove build artifacts
	cd "$(PROJECT_ROOT)" && $(CARGO) clean
	rm -rf "$(PROJECT_ROOT)/src-tauri/gen"
	@echo "Clean complete"

info: ## Print bundle output locations
	@echo ""
	@echo "Build artifacts (platform: $(HOST_OS), Tauri: $(TAURI_MAJOR).x):"
	@if [ -d "$(BUNDLE_DIR)" ]; then \
		find "$(PROJECT_ROOT)/target/release/bundle" -maxdepth 3 -type f 2>/dev/null | head -20 || true; \
		find "$(PROJECT_ROOT)/target/release/bundle" -maxdepth 2 -type d -name "*.app" 2>/dev/null || true; \
	else \
		echo "  (no bundle yet — run make build)"; \
	fi
