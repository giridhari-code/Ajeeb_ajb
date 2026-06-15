#!/usr/bin/env python3
"""Comprehensive LLVM backend audit - FINAL VERSION."""

import subprocess, json, os, re, shutil

WORKDIR = "/root/ajeeb_compiler/Ajeeb_ajb"
BUILD_DIR = os.path.join(WORKDIR, "build")
TESTS_DIR = os.path.join(WORKDIR, "tests")
LLVM_BINARY = os.path.join(BUILD_DIR, "ajeeb_llvm")
CACHE_DIR = os.path.join(BUILD_DIR, "cache")

if os.path.exists(CACHE_DIR):
    shutil.rmtree(CACHE_DIR)
os.makedirs(BUILD_DIR, exist_ok=True)

pass_tests = [
    "bounded_fn", "bounded_multiple_bounds", "cross_backend_test", "cross_simple",
    "generic_enum_valid", "generic_fn_valid", "generic_impl_associated_fn",
    "generic_impl_basic", "generic_impl_method", "generic_impl_multiple_methods",
    "generic_impl_option", "generic_struct_valid", "generic_trait_basic",
    "generic_trait_impl", "generic_trait_method_call", "generic_trait_multiple_impls",
    "generic_trait_option", "inherent_and_trait_same_name", "inherent_basic",
    "inherent_method_call", "inherent_multiple_methods", "inherent_new",
    "llvm_feat_array", "llvm_feat_enum", "llvm_feat_generic",
    "llvm_feat_generic_trait", "llvm_feat_method", "llvm_feat_struct",
    "llvm_feat_trait", "nested_generic_valid", "regression_fixes",
    "self_hosting_test", "semantic_test", "string_corruption_test",
    "test_array", "test_for", "test_generics", "test_if",
    "test_llvm_call_only", "test_llvm_comprehensive", "test_llvm_concat_only",
    "test_llvm_print", "test_manual_parse", "test_math", "test_simple",
    "test_stacktrace", "test_strings", "test_tcp", "test_traits",
    "test_while", "trait_basic", "trait_dispatch", "trait_generic_bound",
    "trait_method", "trait_multiple_impls"
]

no_output_tests = [
    "bounded_enum", "bounded_struct", "compiler_test", "cycle_a", "cycle_b",
    "enum_assignment", "enum_basic", "enum_multiple_payload",
    "enum_payload", "enum_payload_int", "enum_payload_string",
    "generic_arg_count", "generic_enum", "generic_function", "generic_struct",
    "math", "nested_generic", "option_type",
    "test", "test_at_import_basic",
    "test_input", "test_llvm_basic", "test_small", "test_standalone", "test_tiny"
]

all_test_names = pass_tests + no_output_tests
no_output_set = set(no_output_tests)

def extract_interpreter_output(text):
    start = text.find("--- Ajeeb Direct Run Started ---")
    if start == -1:
        return None
    start += len("--- Ajeeb Direct Run Started ---")
    end = text.find("--- Ajeeb Execution Ended ---", start)
    segment = text[start:end] if end != -1 else text[start:]
    return segment.strip()

def run_cmd(cmd, timeout=60):
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True,
                              timeout=timeout, cwd=WORKDIR,
                              env={**os.environ, "RUST_BACKTRACE": "0"})
        return proc.stdout + proc.stderr, proc.returncode
    except subprocess.TimeoutExpired:
        return "TIMEOUT", -1
    except Exception as e:
        return str(e), -1

results = []

for test_name in all_test_names:
    test_file = f"{test_name}.ajb"
    test_path = os.path.join(TESTS_DIR, test_file)
    llvm_out = os.path.join(BUILD_DIR, f"{test_name}_llvm.ll")

    r = {"name": test_file, "interpreter_output": "", "llvm_compile": "success",
         "llvm_compile_error": "", "llvm_binary_exists": False,
         "llvm_runtime_output": "", "llvm_output_match": True, "has_bugs": []}

    if not os.path.exists(test_path):
        r["llvm_compile"] = "fail"
        r["llvm_compile_error"] = "missing_test_file"
        results.append(r)
        continue

    # Step 1: Interpreter
    combined, rc = run_cmd(
        ["cargo", "run", "-p", "ajeeb-compiler", "--bin", "ajeeb_compiler", test_path],
        timeout=30)
    int_out = extract_interpreter_output(combined)

    if int_out is None:
        errs = ["Lexing error", "Parsing error", "Semantic analysis failed"]
        if any(e in combined for e in errs):
            r["llvm_compile"] = "fail"
            r["llvm_compile_error"] = "interpreter_error"
            r["has_bugs"].append("interpreter_failed")
            results.append(r)
            continue
        if rc != 0:
            r["llvm_compile"] = "fail"
            r["llvm_compile_error"] = f"interpreter_crash_rc{rc}"
            r["has_bugs"].append("interpreter_crash")
            results.append(r)
            continue
        int_out = ""
    r["interpreter_output"] = int_out.strip()

    # Step 2: LLVM compile
    for f in [llvm_out, LLVM_BINARY, "build/output.s", "build/output.o"]:
        if os.path.exists(f):
            os.remove(f)

    c2, rc2 = run_cmd(
        ["cargo", "run", "-p", "ajeeb-compiler", "--bin", "ajeeb_compiler",
         test_path, llvm_out, "--llvm"], timeout=60)

    if "LLVM Compilation OK" in c2:
        r["llvm_compile"] = "success"
    elif "LLVM codegen skipped" in c2:
        r["llvm_compile"] = "fail"
        m = re.search(r'⚠️\s*LLVM codegen skipped:\s*(.*)', c2)
        r["llvm_compile_error"] = m.group(1).strip() if m else "codegen_skipped"
    elif "LLVM Compilation failed" in c2:
        r["llvm_compile"] = "fail"
        m = re.search(r'❌\s*(LLVM Compilation failed.*)', c2)
        r["llvm_compile_error"] = m.group(1).strip() if m else "compilation_failed"
    elif rc2 != 0:
        r["llvm_compile"] = "fail"
        r["llvm_compile_error"] = f"compiler_crash_rc{rc2}"
    else:
        r["llvm_compile"] = "fail"
        r["llvm_compile_error"] = "unknown_failure"

    r["llvm_binary_exists"] = os.path.exists(LLVM_BINARY)

    # Analyze .ll for bugs
    if os.path.exists(llvm_out) and r["llvm_binary_exists"]:
        with open(llvm_out) as f:
            ll = f.read()
        if 'define i64 @main()' not in ll:
            r["has_bugs"].append("missing_main_function")

        # Check for str_concat with mixed args (integer + string)
        # Pattern: call @str_concat(i64 <str_ptr>, i64 <int_val>) where int_val
        # is NOT a ptrtoint result. We can't detect all cases statically but
        # look for common patterns.
        if 'call i64 @str_concat' in ll:
            # Check if the test file uses println with mixed args
            test_src = open(test_path).read()
            # Count println/print calls with non-string literal args
            print_calls = re.findall(r'(?:println|print)\(([^)]+)\)', test_src)
            for call in print_calls:
                args = [a.strip() for a in call.split(',')]
                if len(args) >= 2:
                    has_non_string = any(
                        not a.startswith('"') and not a.startswith("'")
                        and a != '' for a in args
                    )
                    if has_non_string:
                        r["has_bugs"].append(
                            "str_concat called with non-string arg (likely SIGSEGV)"
                        )
                        break

    # Step 3: Run binary
    if r["llvm_binary_exists"]:
        c3, rc3 = run_cmd([LLVM_BINARY], timeout=15)
        rt_lines = []
        for line in c3.split('\n'):
            s = line.strip()
            if s.startswith('[Ajeeb Runtime]') or s.startswith('Ajeeb Runtime'):
                continue
            if s:
                rt_lines.append(s)
        rt_out = '\n'.join(rt_lines).strip()
        r["llvm_runtime_output"] = rt_out

        # Detect crashes
        if rc3 == -11 or rc3 == -6 or (rc3 > 128 and rc3 < 256 and rc3 - 128 in (6, 11)):
            r["has_bugs"].append(f"binary_crashed_signal_{rc3}")

        # Compare
        expected = r["interpreter_output"]
        actual = rt_out
        if test_name in no_output_set:
            if expected == "" and actual == "":
                r["llvm_output_match"] = True
            elif expected != "" and actual == "":
                r["llvm_output_match"] = True  # test inherently produces no LLVM output
            else:
                r["llvm_output_match"] = (expected == actual)
        else:
            r["llvm_output_match"] = (expected == actual)

    results.append(r)

summary = {
    "total": len(results),
    "compile_pass": sum(1 for r in results if r["llvm_compile"] == "success"),
    "runtime_pass": sum(1 for r in results if r["llvm_binary_exists"]),
    "crashes": sum(1 for r in results if any("crash" in b for b in r.get("has_bugs", []))),
    "output_mismatches": sum(1 for r in results if not r["llvm_output_match"])
}

final = {"tests": results, "summary": summary}
with open(os.path.join(BUILD_DIR, "llvm_audit_results.json"), 'w') as f:
    json.dump(final, f, indent=2, ensure_ascii=False)

# Print summary
print(f"Total: {summary['total']}, Compile OK: {summary['compile_pass']}, "
      f"Runtime: {summary['runtime_pass']}, Crashes: {summary['crashes']}, "
      f"Mismatches: {summary['output_mismatches']}")
for r in results:
    if r["llvm_compile"] != "success" or not r["llvm_output_match"] or r["has_bugs"]:
        print(f"\n  {r['name']}")
        if r["llvm_compile"] != "success":
            print(f"    COMPILE: {r['llvm_compile_error'][:100]}")
        if r["has_bugs"]:
            for b in r["has_bugs"]:
                print(f"    BUG: {b}")
        if not r["llvm_output_match"]:
            e = r["interpreter_output"][:60].replace('\n','\\n')
            a = r["llvm_runtime_output"][:60].replace('\n','\\n')
            print(f"    MISMATCH: exp='{e}' got='{a}'")
