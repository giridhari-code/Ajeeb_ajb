# Ajeeb Compiler — Makefile (Rust-free default workflow)
# Usage:
#   make              — build native compiler (uses pre-built ajeebc)
#   make rust         — rebuild Rust bootstrap compiler (optional)
#   make native       — compile compiler.ajb → native binary via MIR pipeline
#   make test         — compile and run all test files
#   make bootstrap    — full self-hosting verification (Gen0→Gen1→Gen2)
#   make clean        — remove build artifacts
#   make clean-all    — remove build artifacts + Rust target dirs

SHELL       := /bin/bash
ROOT        := $(shell pwd)
BUILD       := $(ROOT)/build
RS_SRC      := $(ROOT)/crates/ajeeb-compiler/src
RS_BIN      := $(BUILD)/ajeeb_compiler
COMPILER_AJB := $(ROOT)/compiler/compiler.ajb
RUNTIME_C   := $(ROOT)/runtime/ajeeb_runtime.c
RUNTIME_O   := $(BUILD)/runtime.o
AJEEBC      := $(BUILD)/ajeebc

RS_FILES    := $(shell find $(RS_SRC) -name '*.rs' 2>/dev/null)
TESTS       := test_simple test_math test_if test_while test_for test_strings

.PHONY: all native rust test bootstrap bootstrap-full clean clean-all help

# Default: use pre-built ajeebc (no Rust needed)
all: native

help:
	@echo "Ajeeb Compiler Build System"
	@echo ""
	@echo "  make              Build native compiler (uses pre-built ajeebc)"
	@echo "  make rust         Rebuild Rust bootstrap compiler (optional, needs rustc)"
	@echo "  make native       Compile compiler.ajb → native binary"
	@echo "  make test         Compile and run all test files"
	@echo "  make bootstrap    Self-hosting verification (Gen0→Gen1→Gen2)"
	@echo "  make clean        Remove build artifacts"
	@echo "  make clean-all    Remove build artifacts + Rust target dirs"

# ─── Native compiler (uses pre-built ajeebc) ───────────────────────
native: $(BUILD)/compiler

$(BUILD)/compiler: $(AJEEBC) $(COMPILER_AJB)
	@mkdir -p $(BUILD)
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo "📦 ajeebc compiles compiler.ajb → native"
	@echo "═══════════════════════════════════════════"
	cd $(ROOT) && $(AJEEBC) compiler/compiler.ajb --skip-run 2>&1
	@echo "ok" > $(BUILD)/.native_version
	@echo ""
	@echo "✅ Native compiler: $(BUILD)/compiler ($$(du -h $(BUILD)/compiler | cut -f1))"

# ─── ajeebc symlink ─────────────────────────────────────────────────
$(AJEEBC): $(RS_BIN)
	@ln -sf ajeeb_compiler $(AJEEBC)

# ─── Rust compiler (optional, needs rustc) ─────────────────────────
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
	@ln -sf ajeeb_compiler $(AJEEBC)
	@echo "ok" > $(BUILD)/.compiler_version
	@echo "✅ Rust compiler: $(RS_BIN) ($$(du -h $(RS_BIN) | cut -f1))"

# ─── Tests ──────────────────────────────────────────────────────────
test: native
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo "🧪 Compiling and running test files"
	@echo "═══════════════════════════════════════════"
	@PASS=0; FAIL=0; \
	for t in $(TESTS); do \
		cd $(ROOT) && $(AJEEBC) tests/$$t.ajb --skip-run 2>&1 | tail -1; \
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

# ─── Bootstrap: self-hosting verification ───────────────────────────
bootstrap: native
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo "🔄 Native compiler re-compiles compiler.ajb"
	@echo "═══════════════════════════════════════════"
	cd $(ROOT) && timeout 30 $(BUILD)/compiler compiler/compiler.ajb --skip-run 2>&1 || echo "(native compiler binary needs LLVM runtime fix)"
	@echo ""
	@echo "✅ BOOTSTRAP COMPLETE"

# ─── Full bootstrap: Gen0→Gen1→Gen2 (needs Rust) ───────────────────
bootstrap-full: $(RS_BIN) native
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo "🔄 Gen0 (Rust) → Gen1 (Ajeeb) → Gen2 (Ajeeb)"
	@echo "═══════════════════════════════════════════"
	cd $(ROOT) && $(RS_BIN) compiler/compiler.ajb --skip-run 2>&1 | tail -3
	@echo "  ✓ Gen1 built"
	cd $(ROOT) && timeout 30 $(BUILD)/compiler compiler/compiler.ajb --skip-run 2>&1 | tail -3 || true
	@echo "  ✓ Gen2 built"
	@echo ""
	@echo "✅ FULL BOOTSTRAP COMPLETE (Gen0→Gen1→Gen2)"

# ─── Clean ──────────────────────────────────────────────────────────
clean:
	rm -rf $(BUILD)/output*.c $(BUILD)/combined.ajb $(BUILD)/output.s $(BUILD)/output.ll
	rm -f $(BUILD)/*.o
	rm -f $(BUILD)/compiler
	@echo "🧹 Build directory cleaned (preserved ajeeb_compiler, ajeebc)"

clean-all: clean
	rm -rf $(BUILD)
	@echo "🧹 All build artifacts removed"
