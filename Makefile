# Mini TestDisk — cross-platform GUI build (Rust + Tauri 2)
#
# Usage:
#   make help          Show all targets
#   make dev           Run development GUI
#   make build         Build release bundle for current OS
#   make build-macos   Build on macOS (.app / .dmg)
#   make build-linux   Build on Linux (.deb / .AppImage)
#   make build-windows Build on Windows (.exe / .msi)

SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

PROJECT_ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
CARGO        ?= cargo
TAURI        ?= $(CARGO) tauri
NPM          ?= npm

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

.PHONY: help check install-tauri-cli dev build build-debug build-macos build-linux build-windows \
        test test-core sample-image clean info

help: ## Show available targets
	@echo "Mini TestDisk — Makefile targets"
	@echo ""
	@echo "Host OS detected: $(HOST_OS) ($(UNAME_S))"
	@echo ""
	@grep -E '^[a-zA-Z0-9_.-]+:.*##' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

check: ## Verify Rust and Tauri CLI are installed
	@echo "Rust:  $(RUST_VERSION)"
	@echo "Tauri: $(TAURI_VERSION)"
	@echo "Node:  $(NODE_VERSION)"
	@echo "npm:   $(NPM_VERSION)"
	@command -v $(CARGO) >/dev/null 2>&1 || { echo "Error: cargo not found. Install Rust: https://rustup.rs"; exit 1; }
	@command -v node >/dev/null 2>&1 || { echo "Error: node not found. Install Node.js: https://nodejs.org"; exit 1; }
	@command -v $(NPM) >/dev/null 2>&1 || { echo "Error: npm not found. Install Node.js: https://nodejs.org"; exit 1; }
	@$(CARGO) tauri --version >/dev/null 2>&1 || { \
		echo "Error: cargo tauri not found. Run: make install-tauri-cli"; \
		exit 1; \
	}
	@echo "OK — toolchain ready"

install-tauri-cli: ## Install Tauri CLI (cargo install tauri-cli)
	$(CARGO) install tauri-cli --locked

dev: check ## Start GUI in development mode (hot reload backend)
	cd "$(PROJECT_ROOT)" && $(TAURI) dev

build: check ## Build release GUI bundle for current platform
	cd "$(PROJECT_ROOT)" && $(TAURI) build
	@$(MAKE) --no-print-directory info

build-debug: check ## Build debug GUI bundle for current platform
	cd "$(PROJECT_ROOT)" && $(TAURI) build --debug

build-macos: ## Build macOS .app and .dmg (must run on macOS)
ifneq ($(HOST_OS),macos)
	@echo "Error: build-macos requires macOS (current: $(UNAME_S))"
	@exit 1
endif
	cd "$(PROJECT_ROOT)" && $(TAURI) build --bundles app,dmg
	@$(MAKE) --no-print-directory info

build-linux: ## Build Linux .deb and AppImage (must run on Linux)
ifneq ($(HOST_OS),linux)
	@echo "Error: build-linux requires Linux (current: $(UNAME_S))"
	@exit 1
endif
	cd "$(PROJECT_ROOT)" && $(TAURI) build --bundles deb,appimage
	@$(MAKE) --no-print-directory info

build-windows: ## Build Windows .exe and .msi (must run on Windows)
ifneq ($(HOST_OS),windows)
	@echo "Error: build-windows requires Windows (current: $(UNAME_S))"
	@exit 1
endif
	cd "$(PROJECT_ROOT)" && $(TAURI) build --bundles msi,nsis
	@$(MAKE) --no-print-directory info

test: ## Run all Rust unit tests
	cd "$(PROJECT_ROOT)" && $(CARGO) test --workspace

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
	@echo "Build artifacts (platform: $(HOST_OS)):"
	@if [ -d "$(BUNDLE_DIR)" ]; then \
		find "$(PROJECT_ROOT)/target/release/bundle" -maxdepth 3 -type f 2>/dev/null | head -20 || true; \
		find "$(PROJECT_ROOT)/target/release/bundle" -maxdepth 2 -type d -name "*.app" 2>/dev/null || true; \
	else \
		echo "  (no bundle yet — run make build)"; \
	fi
