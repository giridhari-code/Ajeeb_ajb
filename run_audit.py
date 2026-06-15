#!/usr/bin/env python3
"""Comprehensive C backend audit for Ajeeb compiler - optimized version."""
import subprocess, os, json, sys
from pathlib import Path

BASE = Path("/root/ajeeb_compiler/Ajeeb_ajb")
BUILD = BASE / "build"
RUNTIME = BASE / "runtime" / "ajeeb_runtime.c"
BACKEND = BUILD / "ajeeb_native"
TESTS_DIR = BASE / "tests"
INTERP = BASE / "target" / "debug" / "ajeeb_compiler"

PASS_TESTS = [
    "bounded_fn", "bounded_multiple_bounds", "cross_backend_test", "cross_simple",
    "generic_enum_valid", "generic_fn_valid", "generic_impl_associated_fn", "generic_impl_basic",
    "generic_impl_method", "generic_impl_multiple_methods", "generic_impl_option", "generic_struct_valid",
    "generic_trait_basic", "generic_trait_impl", "generic_trait_method_call", "generic_trait_multiple_impls",
    "generic_trait_option", "inherent_and_trait_same_name", "inherent_basic", "inherent_method_call",
    "inherent_multiple_methods", "inherent_new", "llvm_feat_array", "llvm_feat_enum", "llvm_feat_generic",
    "llvm_feat_generic_trait", "llvm_feat_method", "llvm_feat_struct", "llvm_feat_trait", "nested_generic_valid",
    "regression_fixes", "self_hosting_test", "semantic_test", "string_corruption_test", "test_array", "test_for",
    "test_generics", "test_if", "test_llvm_call_only", "test_llvm_comprehensive", "test_llvm_concat_only",
    "test_llvm_print", "test_manual_parse", "test_math", "test_simple", "test_stacktrace", "test_strings",
    "test_tcp", "test_traits", "test_while", "trait_basic", "trait_dispatch", "trait_generic_bound", "trait_method",
    "trait_multiple_impls"
]

NO_OUTPUT_TESTS = [
    "bounded_enum", "bounded_struct", "compiler_test", "cycle_a", "cycle_b", "enum_assignment", "enum_basic",
    "enum_compare", "enum_multiple_payload", "enum_payload", "enum_payload_int", "enum_payload_string",
    "generic_arg_count", "generic_enum", "generic_function", "generic_struct", "math", "nested_generic",
    "option_type", "struct_basic", "struct_field_access", "struct_field_assign", "struct_literal",
    "struct_nested", "struct_verify", "test", "test_at_import_basic", "test_input", "test_llvm_basic",
    "test_small", "test_standalone", "test_tiny"
]

ALL_TESTS = PASS_TESTS + NO_OUTPUT_TESTS

# Tests known to hang
SKIP_TESTS = {"test_tcp", "test_input", "test_standalone"}

def extract_interpreter_output(stdout):
    start = stdout.find("--- Ajeeb Direct Run Started ---")
    end = stdout.find("--- Ajeeb Execution Ended ---")
    if start == -1 or end == -1:
        return stdout.strip()
    return stdout[start+len("--- Ajeeb Direct Run Started ---"):end].strip()

results = []
summary = {
    "total": len(ALL_TESTS),
    "compile_pass": 0,
    "runtime_pass": 0,
    "crashes": 0,
    "output_mismatches": 0
}

for idx, test_name in enumerate(ALL_TESTS):
    ajb_file = f"{test_name}.ajb"
    ajb_path = TESTS_DIR / ajb_file
    c_file = f"build/{test_name}_c.c"
    c_bin = f"build/{test_name}_c_bin"
    c_path = BASE / c_file
    c_bin_path = BASE / c_bin

    # Cleanup
    for p in [c_path, c_bin_path]:
        p.unlink(missing_ok=True)

    result = {
        "name": ajb_file,
        "interpreter_output": "",
        "c_compile": "fail",
        "c_compile_error": "",
        "c_binary_runs": False,
        "c_runtime_output": "",
        "c_output_match": False,
        "has_bugs": []
    }

    if test_name in SKIP_TESTS:
        result["has_bugs"].append(f"Skipped - test known to hang/require input")
        results.append(result)
        print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: SKIPPED (known hang)")
        continue

    # Step 1: Run interpreter (10s timeout)
    try:
        r = subprocess.run([str(INTERP), str(ajb_path)], capture_output=True, text=True, timeout=10, cwd=BASE)
        interp_out = extract_interpreter_output(r.stdout)
    except subprocess.TimeoutExpired:
        result["has_bugs"].append("Interpreter timed out")
        result["interpreter_output"] = "(TIMEOUT)"
        results.append(result)
        print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: INTERPRETER TIMEOUT")
        continue
    except Exception as e:
        result["has_bugs"].append(f"Interpreter error: {e}")
        results.append(result)
        print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: INTERPRETER ERROR: {e}")
        continue

    result["interpreter_output"] = interp_out

    # Step 2: Compile via C backend (15s timeout)
    try:
        r2 = subprocess.run([str(BACKEND), str(ajb_path), str(c_path)],
                           capture_output=True, text=True, timeout=15, cwd=BASE)
        if r2.returncode == 0 and c_path.exists() and c_path.stat().st_size > 0:
            result["c_compile"] = "success"
            summary["compile_pass"] += 1
        else:
            result["c_compile"] = "fail"
            result["c_compile_error"] = (r2.stdout + r2.stderr).strip()[:500]
            result["has_bugs"].append("C backend failed to generate code")
            results.append(result)
            print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: C BACKEND FAIL")
            continue
    except subprocess.TimeoutExpired:
        result["has_bugs"].append("C backend timed out")
        results.append(result)
        print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: C BACKEND TIMEOUT")
        continue

    # Step 3: Compile C code with GCC (15s timeout)
    try:
        r3 = subprocess.run(["gcc", "-w", "-o", str(c_bin_path), str(c_path), str(RUNTIME)],
                           capture_output=True, text=True, timeout=15, cwd=BASE)
        if r3.returncode != 0:
            result["c_compile"] = "fail"
            result["c_compile_error"] = (r3.stdout + r3.stderr).strip()[:500]
            result["has_bugs"].append("GCC compilation failed")
            results.append(result)
            print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: GCC FAIL")
            continue
    except subprocess.TimeoutExpired:
        result["has_bugs"].append("GCC compilation timed out")
        results.append(result)
        print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: GCC TIMEOUT")
        continue

    # Step 4: Run binary (10s timeout)
    if not c_bin_path.exists():
        result["has_bugs"].append("Binary not found after GCC compilation")
        results.append(result)
        print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: BINARY MISSING")
        continue

    try:
        r4 = subprocess.run([str(c_bin_path)], capture_output=True, text=True, timeout=10, cwd=BASE)
        runtime_out = r4.stdout.strip()
        if r4.returncode != 0:
            result["has_bugs"].append(f"Binary crashed with exit code {r4.returncode}: {r4.stderr.strip()[:200]}")
            summary["crashes"] += 1
        else:
            result["c_binary_runs"] = True
            summary["runtime_pass"] += 1
    except subprocess.TimeoutExpired:
        runtime_out = ""
        result["has_bugs"].append("Binary timed out (possible infinite loop)")
        summary["crashes"] += 1
        results.append(result)
        print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: BINARY TIMEOUT")
        continue

    result["c_runtime_output"] = runtime_out

    # Step 5: Compare output
    if interp_out == runtime_out:
        result["c_output_match"] = True
    else:
        result["c_output_match"] = False
        summary["output_mismatches"] += 1
        result["has_bugs"].append(
            f"Output mismatch. Interpreter: '{interp_out[:200]}', C: '{runtime_out[:200]}'"
        )

    results.append(result)
    print(f"[{idx+1}/{len(ALL_TESTS)}] {ajb_file}: compile={result['c_compile']}, runs={result['c_binary_runs']}, match={result['c_output_match']}")

# Write results
output = {"tests": results, "summary": summary}
output_path = BASE / "build" / "c_audit_results.json"
output_path.write_text(json.dumps(output, indent=2))

print(f"\n{'='*60}")
print(f"Results written to {output_path}")
print(f"Summary: {json.dumps(summary, indent=2)}")
print(f"{'='*60}")
