#!/bin/bash
# Final comprehensive C backend audit
# Processes all tests and saves incremental results

BASE="/root/ajeeb_compiler/Ajeeb_ajb"
INTERP="$BASE/target/debug/ajeeb_compiler"
BACKEND="$BASE/build/ajeeb_native"
RUNTIME="$BASE/runtime/ajeeb_runtime.c"
JSON="$BASE/build/c_audit_results.json"
PARTIAL="$BASE/build/c_audit_partial.json"

ALL_TESTS=(
bounded_fn bounded_multiple_bounds cross_backend_test cross_simple
generic_enum_valid generic_fn_valid generic_impl_associated_fn generic_impl_basic
generic_impl_method generic_impl_multiple_methods generic_impl_option generic_struct_valid
generic_trait_basic generic_trait_impl generic_trait_method_call generic_trait_multiple_impls
generic_trait_option inherent_and_trait_same_name inherent_basic inherent_method_call
inherent_multiple_methods inherent_new llvm_feat_array llvm_feat_enum llvm_feat_generic
llvm_feat_generic_trait llvm_feat_method llvm_feat_struct llvm_feat_trait nested_generic_valid
regression_fixes self_hosting_test semantic_test string_corruption_test test_array test_for
test_generics test_if test_llvm_call_only test_llvm_comprehensive test_llvm_concat_only
test_llvm_print test_manual_parse test_math test_simple test_stacktrace test_strings
test_tcp test_traits test_while trait_basic trait_dispatch trait_generic_bound trait_method
trait_multiple_impls
bounded_enum bounded_struct compiler_test cycle_a cycle_b enum_assignment enum_basic
enum_compare enum_multiple_payload enum_payload enum_payload_int enum_payload_string
generic_arg_count generic_enum generic_function generic_struct math nested_generic
option_type struct_basic struct_field_access struct_field_assign struct_literal
struct_nested struct_verify test test_at_import_basic test_input test_llvm_basic
test_small test_standalone test_tiny
)

TOTAL=${#ALL_TESTS[@]}
echo "Total tests: $TOTAL"

# Initialize JSON
echo '{"tests":[],"summary":{"total":0,"compile_pass":0,"runtime_pass":0,"crashes":0,"output_mismatches":0}}' > "$JSON"

for ((i=0; i<TOTAL; i++)); do
  tn="${ALL_TESTS[$i]}"
  AJB="$BASE/tests/$tn.ajb"
  CFILE="$BASE/build/${tn}_c.c"
  CBIN="$BASE/build/${tn}_c_bin"
  
  rm -f "$CFILE" "$CBIN" "$BASE/build/output.c" "$BASE/build/output2.c"
  
  echo "--- [$((i+1))/$TOTAL] $tn ---"
  
  # JSON entry builder
  entry="{\"name\":\"$tn.ajb\""
  
  # Step 1: Interpreter
  INTERP_OUT=$(timeout 10 "$INTERP" "$AJB" 2>/dev/null)
  IP_EXIT=$?
  if [ $IP_EXIT -eq 124 ]; then
    INTERP_OUT="(TIMEOUT)"
    entry+=",\"interpreter_output\":\"(TIMEOUT)\""
  elif [ $IP_EXIT -ne 0 ]; then
    entry+=",\"interpreter_output\":\"(ERROR:$IP_EXIT)\""
  else
    # Extract between markers
    EXTRACTED=$(echo "$INTERP_OUT" | sed -n '/--- Ajeeb Direct Run Started ---/,/--- Ajeeb Execution Ended ---/p' | sed '1d;$d')
    # Escape for JSON
    EXTRACTED_ESC=$(echo "$EXTRACTED" | python3 -c "import sys,json; print(json.dumps(sys.stdin.read().strip()))" 2>/dev/null || echo "\"\"")
    entry+=",\"interpreter_output\":$EXTRACTED_ESC"
  fi
  
  # Step 2: C backend
  C_OUT=$(timeout 15 "$BACKEND" "$AJB" "$CFILE" 2>&1)
  C_EXIT=$?
  if [ $C_EXIT -eq 0 ] && [ -f "$CFILE" ] && [ -s "$CFILE" ]; then
    entry+=",\"c_compile\":\"success\",\"c_compile_error\":\"\""
  else
    ERR_ESC=$(echo "$C_OUT" | head -c 500 | python3 -c "import sys,json; print(json.dumps(sys.stdin.read()))" 2>/dev/null || echo "\"\"")
    entry+=",\"c_compile\":\"fail\",\"c_compile_error\":$ERR_ESC"
    entry+=",\"c_binary_runs\":false,\"c_runtime_output\":\"\",\"c_output_match\":false,\"has_bugs\":[\"C backend failed to generate code\"]}"
    echo "$entry" | python3 -c "
import json,sys
d=json.loads(sys.stdin.read())
with open('$JSON','r') as f: r=json.load(f)
r['tests'].append(d)
r['summary']={'total':len(r['tests']),'compile_pass':sum(1 for t in r['tests'] if t['c_compile']=='success'),'runtime_pass':sum(1 for t in r['tests'] if t['c_binary_runs']),'crashes':sum(1 for t in r['tests'] if any('crash' in b for b in t.get('has_bugs',[]))),'output_mismatches':sum(1 for t in r['tests'] if not t['c_output_match'])}
with open('$JSON','w') as f: json.dump(r,f,indent=2)
"
    echo "  C BACKEND FAIL"
    continue
  fi
  
  # Step 3: GCC
  GCC_OUT=$(timeout 15 gcc -w -o "$CBIN" "$CFILE" "$RUNTIME" 2>&1)
  GCC_EXIT=$?
  if [ $GCC_EXIT -ne 0 ]; then
    ERR_ESC=$(echo "$GCC_OUT" | head -c 500 | python3 -c "import sys,json; print(json.dumps(sys.stdin.read()))" 2>/dev/null || echo "\"\"")
    entry+=",\"c_compile\":\"fail\",\"c_compile_error\":$ERR_ESC"
    entry+=",\"c_binary_runs\":false,\"c_runtime_output\":\"\",\"c_output_match\":false,\"has_bugs\":[\"GCC compilation failed\"]}"
    echo "$entry" | python3 -c "
import json,sys
d=json.loads(sys.stdin.read())
with open('$JSON','r') as f: r=json.load(f)
r['tests'].append(d)
r['summary']={'total':len(r['tests']),'compile_pass':sum(1 for t in r['tests'] if t['c_compile']=='success'),'runtime_pass':sum(1 for t in r['tests'] if t['c_binary_runs']),'crashes':sum(1 for t in r['tests'] if any('crash' in b for b in t.get('has_bugs',[]))),'output_mismatches':sum(1 for t in r['tests'] if not t['c_output_match'])}
with open('$JSON','w') as f: json.dump(r,f,indent=2)
"
    echo "  GCC FAIL"
    continue
  fi
  
  # Step 4: Run binary
  BIN_OUT=$(timeout 10 "$CBIN" 2>&1)
  BIN_EXIT=$?
  RO=$(echo "$BIN_OUT" | python3 -c "import sys,json; print(json.dumps(sys.stdin.read().strip()))" 2>/dev/null || echo "\"\"")
  
  if [ $BIN_EXIT -eq 124 ]; then
    entry+=",\"c_binary_runs\":false,\"c_runtime_output\":\"(TIMEOUT)\""
    entry+=",\"c_output_match\":false,\"has_bugs\":[\"Binary timed out\"]}"
  elif [ $BIN_EXIT -ne 0 ]; then
    entry+=",\"c_binary_runs\":false,\"c_runtime_output\":$RO"
    entry+=",\"c_output_match\":false,\"has_bugs\":[\"Binary crashed exit=$BIN_EXIT\"]}"
  else
    entry+=",\"c_binary_runs\":true,\"c_runtime_output\":$RO"
    # Compare
    entry+=",\"c_output_match\":"
    # Get interpreter output for comparison
    IO=$(echo "$entry" | python3 -c "import json,sys; print(json.loads(sys.stdin.read())['interpreter_output'])" 2>/dev/null)
    # python3 compare
  fi
  
  # Final comparison and save
  echo "$entry" | python3 -c "
import json,sys
d=json.loads(sys.stdin.read())
# Get interpreter output for comparison
io=d.get('interpreter_output','')
ro=d.get('c_runtime_output','')
if d.get('c_binary_runs',False):
    d['c_output_match'] = (io==ro)
else:
    d['c_output_match'] = False

# Check for bugs
if not d.get('has_bugs'):
    d['has_bugs']=[]
if d.get('c_binary_runs',False) and not d['c_output_match']:
    d['has_bugs'].append(f\"Output mismatch. Interpreter: '{io[:100]}', C: '{ro[:100]}'\")

with open('$JSON','r') as f: r=json.load(f)
r['tests'].append(d)
r['summary']={'total':len(r['tests']),'compile_pass':sum(1 for t in r['tests'] if t['c_compile']=='success'),'runtime_pass':sum(1 for t in r['tests'] if t['c_binary_runs']),'crashes':sum(1 for t in r['tests'] if any('crash' in b for b in t.get('has_bugs',[]))),'output_mismatches':sum(1 for t in r['tests'] if not t['c_output_match'])}
with open('$JSON','w') as f: json.dump(r,f,indent=2)
print('OK')
" 2>/dev/null
  
  # Summary line
  CSTAT=$(python3 -c "import json; r=json.load(open('$JSON')); t=r['tests'][-1]; print(f\"compile={t['c_compile']} runs={t['c_binary_runs']} match={t['c_output_match']}\")")
  echo "  $CSTAT"
done

echo ""
echo "=== FINAL SUMMARY ==="
python3 -c "import json; print(json.dumps(json.load(open('$JSON'))['summary'], indent=2))"
echo "Results: $JSON"
