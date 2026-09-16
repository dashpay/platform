#!/bin/bash
# Exercise CI setup with stub commands; never access a real keychain.
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
test_dir="$(mktemp -d)"
trap 'rm -rf "$test_dir"' EXIT
mkdir "$test_dir/bin"
cat > "$test_dir/bin/security" <<'STUB'
#!/bin/bash
printf '%s\n' "$*" >> "$CALL_LOG"
case "$*" in
  'default-keychain -d user')
    if [ -f "$KEYCHAIN_STATE" ]; then
      cat "$KEYCHAIN_STATE"
    elif [ "$LOOKUP_MODE" = absent ]; then
      echo 'security: SecKeychainCopyDomainDefault user: A default keychain could not be found.' >&2
      exit 1
    elif [ "$LOOKUP_MODE" = unexpected ]; then
      echo 'security: SecKeychainCopyDomainDefault user: A default keychain could not be found.' >&2
      exit 42
    elif [ "$LOOKUP_MODE" = denied ]; then
      echo 'security: SecKeychainCopyDomainDefault user: User interaction is not allowed.' >&2
      exit 1
    else
      echo '    "/saved/login.keychain-db"'
    fi ;;
  'default-keychain -d user -s '*) printf '%s\n' "$5" > "$KEYCHAIN_STATE" ;;
  'list-keychains -d user') echo '    "/saved/login.keychain-db"' ;;
  'find-generic-password '*) echo writable ;;
esac
STUB
cat > "$test_dir/bin/xcrun" <<'STUB'
#!/bin/bash
# Stop after setup, before any builds.
echo "iPhone 16 (test-device)"
exit 42
STUB
chmod +x "$test_dir/bin/"*
for mode in denied unexpected absent present; do
  export LOOKUP_MODE="$mode" CALL_LOG="$test_dir/$mode.calls" KEYCHAIN_STATE="$test_dir/$mode.state"
  status=0
  CI=1 SIM_NAME='' PATH="$test_dir/bin:$PATH" RUNNER_TEMP="$test_dir" \
    bash "$script_dir/../run_tests.sh" > "$test_dir/$mode.output" 2>&1 || status=$?
  if [ "$mode" = denied ] || [ "$mode" = unexpected ]; then
    expected_status=1
    if [ "$mode" = unexpected ]; then expected_status=42; fi
    if [ "$status" -ne "$expected_status" ] || [ "$(wc -l < "$CALL_LOG" | tr -d ' ')" -ne 1 ]; then
      echo 'FAIL: unexpected lookup failure must abort before further keychain operations' >&2
      cat "$test_dir/$mode.output" >&2
      exit 1
    fi
    grep -q 'security: SecKeychainCopyDomainDefault user:' "$test_dir/$mode.output"
  else
    [ "$status" -eq 42 ]
    grep -q '^create-keychain ' "$CALL_LOG"
    grep -q '^delete-keychain ' "$CALL_LOG"
    if [ "$mode" = present ]; then
      grep -q '^default-keychain -d user -s /saved/login.keychain-db$' "$CALL_LOG"
    else
      [ "$(grep -c '^default-keychain -d user -s ' "$CALL_LOG")" -eq 1 ]
    fi
  fi
  echo "PASS: $mode"
done
