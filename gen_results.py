#!/usr/bin/env python3
"""Generate final corrected audit JSON with accurate classifications."""
import json
from pathlib import Path

BASE = Path("/root/ajeeb_compiler/Ajeeb_ajb")
BUILD = BASE / "build"
RUNTIME = BASE / "runtime" / "ajeeb_runtime.c"
BACKEND = BUILD / "ajeeb_native"
TESTS_DIR = BASE / "tests"
INTERP = BASE / "target" / "debug" / "ajeeb_compiler"

ALL_TESTS = [
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
    "trait_multiple_impls",
    "bounded_enum", "bounded_struct", "compiler_test", "cycle_a", "cycle_b", "enum_assignment", "enum_basic",
    "enum_compare", "enum_multiple_payload", "enum_payload", "enum_payload_int", "enum_payload_string",
    "generic_arg_count", "generic_enum", "generic_function", "generic_struct", "math", "nested_generic",
    "option_type", "struct_basic", "struct_field_access", "struct_field_assign", "struct_literal",
    "struct_nested", "struct_verify", "test", "test_at_import_basic", "test_input", "test_llvm_basic",
    "test_small", "test_standalone", "test_tiny"
]

def extract_interp(out):
    start = out.find("--- Ajeeb Direct Run Started ---")
    end = out.find("--- Ajeeb Execution Ended ---")
    if start >= 0 and end >= 0:
        return out[start+len("--- Ajeeb Direct Run Started ---"):end].strip()
    return out.strip()

def run(cmd, timeout=15):
    import subprocess
    try:
        r = subprocess.run(["timeout", str(timeout)] + cmd, capture_output=True, text=True, timeout=timeout+5, cwd=BASE)
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        return 124, "", ""

def check_c_file(name):
    """Check generated C code for issues."""
    cfile = BUILD / f"{name}_c.c"
    if not cfile.exists():
        return []
    content = cfile.read_text()
    bugs = []
    lines = content.split("\n")
    
    # Check for raw AST dump (invalid C)
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped in ("trait;", "enum;", "struct;", "impl;") or stripped.startswith("trait ") and stripped.endswith(";"):
            bugs.append(f"Raw AST dump at line {i+1}: '{stripped}' (not valid C)")
            break
    # Check for bare integer/string on its own line (AST leak)
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped and not stripped.startswith(("//", "#", "int", "void", "char", "bool", "return", "if", "for", "while", "}", "{", "    }", "    {", "    if", "    for", "    while", "    return", "    int", "    char", "    void", "    println", "    print", "    break", "    continue", "    switch", "    case", "    default", "float", "double", "unsigned", "static", "struct ", "enum ", "union ", "typedef", "#define", "#include", "#if", "#ifdef", "#ifndef", "#else", "#endif", "extern", "inline", "const", "volatile", "register", "signed", "short", "long", "auto")):
            # Could be a stray AST element
            if any(c.isalpha() for c in stripped[:3]) and not stripped.startswith("__") and not stripped.startswith("intptr_t"):
                bugs.append(f"Possible AST leak at line {i+1}: '{stripped[:80]}'")
                break
    
    if "Hello from LLVM!" in content:
        bugs.append("Contains default 'Hello from LLVM!' placeholder instead of actual generated code")
    
    if content.count("int main(") == 0:
        bugs.append("No main() function found")
    
    return bugs

results = []
summary = {"total": len(ALL_TESTS), "c_backend_success": 0, "gcc_compile_fail": 0, "runtime_pass": 0, "segfaults": 0, "output_mismatches": 0}

for idx, tn in enumerate(ALL_TESTS):
    ajb = TESTS_DIR / f"{tn}.ajb"
    cfile = BUILD / f"{tn}_c.c"
    cbin = BUILD / f"{tn}_c_bin"

    r = {"name": f"{tn}.ajb", "interpreter_output": "", "c_compile": "fail",
         "c_compile_error": "", "c_binary_runs": False, "c_runtime_output": "",
         "c_output_match": False, "has_bugs": []}

    # Step 1: Interpreter
    rc, out, _ = run([str(INTERP), str(ajb)], 10)
    if rc == 124:
        r["interpreter_output"] = "(TIMEOUT)"
        r["has_bugs"].append("Interpreter timed out")
    elif rc != 0:
        r["interpreter_output"] = f"(ERROR {rc})"
    else:
        r["interpreter_output"] = extract_interp(out)

    # Step 2: C backend
    if not cfile.exists() or cfile.stat().st_size == 0:
        rc, sout, serr = run([str(BACKEND), str(ajb), str(cfile)], 15)
        if (not cfile.exists() or cfile.stat().st_size == 0):
            r["c_compile_error"] = (sout + serr).strip()[:300]
            r["has_bugs"].append("C backend failed to produce output file")
            results.append(r)
            print(f"[{idx+1}/{len(ALL_TESTS)}] {tn}: BACKEND FAIL")
            continue

    if not cfile.exists() or cfile.stat().st_size == 0:
        r["has_bugs"].append("C backend produced empty output")
        results.append(r)
        print(f"[{idx+1}/{len(ALL_TESTS)}] {tn}: NO C FILE")
        continue

    r["c_compile"] = "success"
    summary["c_backend_success"] += 1

    # Check C code quality
    for bug in check_c_file(tn):
        r["has_bugs"].append(bug)

    # Step 3: GCC
    rc, _, serr = run(["gcc", "-w", "-o", str(cbin), str(cfile), str(RUNTIME)], 15)
    if rc != 0:
        r["c_compile"] = "fail"
        r["c_compile_error"] = serr.strip()[:400]
        r["has_bugs"].append("GCC compilation failed")
        if not any("Raw AST dump" in b for b in r["has_bugs"]):
            r["has_bugs"].append("Generated C code contains syntax errors")
        summary["gcc_compile_fail"] += 1
        results.append(r)
        fstat = "GCC FAIL"
        if any("Raw AST" in b for b in r["has_bugs"]):
            fstat += " (AST dump)"
        print(f"[{idx+1}/{len(ALL_TESTS)}] {tn}: {fstat}")
        continue

    # Step 4: Run binary
    rc, bout, berr = run([str(cbin)], 10)
    r["c_runtime_output"] = bout.strip()

    if rc == 124:
        r["has_bugs"].append("Binary timed out")
    elif rc == -11 or rc == 139:  # SIGSEGV (139 = 128+11)
        r["c_binary_runs"] = False
        r["has_bugs"].append(f"Binary segfault (SIGSEGV)")
        summary["segfaults"] += 1
    elif rc != 0:
        # Non-zero exit - could be program behavior (like test_math returning 42)
        # We still consider the binary as running, but note the exit code
        r["c_binary_runs"] = True
        r["has_bugs"].append(f"Binary ran but exited with code {rc}")
        summary["runtime_pass"] += 1
    else:
        r["c_binary_runs"] = True
        summary["runtime_pass"] += 1

    # Compare output (only if binary ran and we have interpreter output)
    io = r["interpreter_output"]
    co = r["c_runtime_output"]
    if r["c_binary_runs"]:
        if io == co:
            r["c_output_match"] = True
        else:
            r["c_output_match"] = False
            summary["output_mismatches"] += 1
            if not any("Output mismatch" in b for b in r["has_bugs"]):
                r["has_bugs"].append(f"Output mismatch. Interp: '{io[:120]}', C: '{co[:120]}'")

    results.append(r)
    stats = f"compile={r['c_compile']}, runs={r['c_binary_runs']}, match={r['c_output_match']}"
    print(f"[{idx+1}/{len(ALL_TESTS)}] {tn}: {stats}")

# Write results
output = {"tests": results, "summary": summary}
outpath = BASE / "build" / "c_audit_results.json"
outpath.write_text(json.dumps(output, indent=2))

print(f"\n{'='*60}")
print(f"Results: {outpath}")
print(f"Summary: {json.dumps(summary, indent=2)}")
print(f"{'='*60}")
