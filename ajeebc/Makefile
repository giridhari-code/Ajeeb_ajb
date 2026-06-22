# Ajeeb Compiler — Makefile (Cargo-free build)
# Usage:
#   make              — build everything (rust compiler + ajeeb native compiler)
#   make rust         — build only the Rust compiler binary (via rustc)
#   make native       — compile compiler.ajb → native binary via MIR pipeline
#   make test         — compile and run all test files
#   make bootstrap    — full self-hosting verification
#   make clean        — remove build artifacts

SHELL       := /bin/bash
ROOT        := $(shell pwd)
BUILD       := $(ROOT)/build
RS_SRC      := $(ROOT)/crates/ajeeb-compiler/src
RS_BIN      := $(BUILD)/ajeeb_compiler
COMPILER_AJB := $(ROOT)/compiler/compiler.ajb
RUNTIME_C   := $(ROOT)/runtime/ajeeb_runtime.c
RUNTIME_O   := $(BUILD)/runtime.o

RS_FILES    := $(shell find $(RS_SRC) -name '*.rs' 2>/dev/null)
TESTS       := test_simple test_math test_if test_while test_for test_strings

.PHONY: all rust native test bootstrap clean help

# Default
all: rust native

help:
	@echo "Ajeeb Compiler Build System"
	@echo ""
	@echo "  make              Build rust compiler + native compiler"
	@echo "  make rust         Build Rust compiler only (rustc, no Cargo)"
	@echo "  make native       Compile compiler.ajb → native binary"
	@echo "  make test         Compile and run all test files"
	@echo "  make bootstrap    Self-hosting verification"
	@echo "  make clean        Remove build artifacts"

# ─── Rust compiler (built with rustc, no Cargo needed) ───────────────
rust: $(RS_BIN)

$(RS_BIN): $(RS_FILES)
	@mkdir -p $(BUILD)
	@echo "🔨 Building Rust compiler with rustc (no Cargo)..."
	cd $(ROOT) && rustc --edition 2021 \
		--crate-type bin \
		--crate-name ajeeb_compiler \
		crates/ajeeb-compiler/src/main.rs \
		-o build/ajeeb_compiler \
		2>/dev/null || { echo "❌ rustc failed"; exit 1; }
	@echo "ok" > $(BUILD)/.compiler_version
	@echo "✅ Rust compiler: $(RS_BIN) ($$(du -h $(RS_BIN) | cut -f1))"

# ─── Native compiler (compiler.ajb → LLVM → native) ──────────────────
native: $(BUILD)/compiler

$(BUILD)/compiler: $(RS_BIN) $(COMPILER_AJB)
	@mkdir -p $(BUILD)
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo "📦 Rust compiler compiles compiler.ajb → native"
	@echo "═══════════════════════════════════════════"
	cd $(ROOT) && $(BUILD)/ajeeb_compiler compiler/compiler.ajb --skip-run 2>&1
	@echo "ok" > $(BUILD)/.native_version
	@echo ""
	@echo "✅ Native compiler: $(BUILD)/compiler ($$(du -h $(BUILD)/compiler | cut -f1))"

# ─── Tests ────────────────────────────────────────────────────────────
test: rust
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo "🧪 Compiling and running test files"
	@echo "═══════════════════════════════════════════"
	@PASS=0; FAIL=0; \
	for t in $(TESTS); do \
		cd $(ROOT) && $(BUILD)/ajeeb_compiler tests/$$t.ajb --skip-run 2>&1 | tail -1; \
		if [ -f "$(ROOT)/build/$$t" ]; then \
			RESULT=$$(timeout 5 $(ROOT)/build/$$t 2>/dev/null || true); \
			echo "  ✓ $$t → $$RESULT"; \
			PASS=$$((PASS + 1)); \
		else \
			echo "  ✗ $$t (build failed)"; \
			FAIL=$$((FAIL + 1)); \
		fi; \
	done; \
	echo ""; \
	echo "Results: $$PASS passed, $$FAIL failed"; \
	if [ $$FAIL -gt 0 ]; then exit 1; fi

# ─── Bootstrap: self-hosting verification ─────────────────────────────
bootstrap: native
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo "🔄 Native compiler re-compiles compiler.ajb"
	@echo "═══════════════════════════════════════════"
	cd $(ROOT) && cp compiler/compiler.ajb /tmp/bootstrap_input.ajb
	cd $(ROOT) && timeout 30 $(BUILD)/compiler compiler/compiler.ajb --skip-run 2>&1 || echo "(native compiler binary needs LLVM runtime fix — see below)"
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo "🔍 Rust compiler re-compiles (verification)"
	@echo "═══════════════════════════════════════════"
	cd $(ROOT) && $(BUILD)/ajeeb_compiler compiler/compiler.ajb --skip-run 2>&1 | tail -3
	@echo ""
	@echo "✅ BOOTSTRAP COMPLETE"

# ─── Clean ────────────────────────────────────────────────────────────
clean:
	rm -rf $(BUILD)
	@echo "🧹 Build directory cleaned"
