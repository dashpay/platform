#!/bin/bash
set -euo pipefail

fake_log() {
  printf '%s\n' "$1" >> "$FAKE_SECURITY_STATE/command.log"
}

fake_fail_once() {
  expected_failure="$1"
  configured_failure="$(cat "$FAKE_SECURITY_STATE/fail_at")"
  if [ "$configured_failure" = "$expected_failure" ] \
    && [ ! -e "$FAKE_SECURITY_STATE/failure_used" ]; then
    : > "$FAKE_SECURITY_STATE/failure_used"
    return 0
  fi
  return 1
}

fake_is_managed_keychain() {
  candidate_path="$1"
  managed_root="$(cat "$FAKE_SECURITY_STATE/managed_root")"
  case "$candidate_path" in
    "$managed_root"/run.*/tests.keychain-db)
      return 0
      ;;
  esac
  return 1
}

fake_is_current_keychain() {
  candidate_path="$1"
  current_keychain="$(cat "$FAKE_SECURITY_STATE/current_keychain")"
  [ -n "$current_keychain" ] && [ "$candidate_path" = "$current_keychain" ]
}

fake_require_current_keychain() {
  candidate_path="$1"
  if ! fake_is_current_keychain "$candidate_path"; then
    echo "fake security rejected mutation of seeded keychain: $candidate_path" >&2
    return 90
  fi
}

fake_security_list_query() {
  count="$(cat "$FAKE_SECURITY_STATE/list_query_count")"
  count=$((count + 1))
  printf '%s\n' "$count" > "$FAKE_SECURITY_STATE/list_query_count"

  if [ "$count" -eq 1 ]; then
    initial_mode="$(cat "$FAKE_SECURITY_STATE/initial_list_mode")"
    case "$initial_mode" in
      exit37)
        cat "$FAKE_SECURITY_STATE/initial_list_output"
        return 37
        ;;
      exit44)
        cat "$FAKE_SECURITY_STATE/initial_list_output"
        return 44
        ;;
      error61)
        cat "$FAKE_SECURITY_STATE/initial_list_output"
        return 61
        ;;
      ok)
        ;;
      *)
        echo "unknown fake list mode: $initial_mode" >&2
        return 92
        ;;
    esac
  fi

  if [ "$count" -gt 1 ]; then
    current_keychain="$(cat "$FAKE_SECURITY_STATE/current_keychain")"
    if [ -n "$current_keychain" ]; then
      if fake_fail_once cleanup_list_readback; then
        return 73
      fi
    elif fake_fail_once repair_list_readback; then
      return 73
    fi
  fi

  while IFS= read -r keychain_path; do
    printf '    "%s"\n' "$keychain_path"
  done < "$FAKE_SECURITY_STATE/search_list"
}

fake_security_list_set() {
  shift 4
  if [ "$#" -eq 0 ]; then
    echo "fake security does not support an empty user search list" >&2
    return 91
  fi

  first_path="$1"
  list_kind=repair
  current_keychain="$(cat "$FAKE_SECURITY_STATE/current_keychain")"
  if fake_is_current_keychain "$first_path"; then
    list_kind=temp
  elif [ -n "$current_keychain" ]; then
    list_kind=cleanup
  fi

  if fake_fail_once "${list_kind}_list_set_before"; then
    return 73
  fi

  : > "$FAKE_SECURITY_STATE/search_list"
  for keychain_path in "$@"; do
    printf '%s\n' "$keychain_path" >> "$FAKE_SECURITY_STATE/search_list"
  done
  fake_log "list-set-$list_kind"

  if fake_fail_once "${list_kind}_list_set_after"; then
    return 73
  fi
}

fake_security_default_query() {
  count="$(cat "$FAKE_SECURITY_STATE/default_query_count")"
  count=$((count + 1))
  printf '%s\n' "$count" > "$FAKE_SECURITY_STATE/default_query_count"

  if [ "$count" -eq 1 ]; then
    initial_mode="$(cat "$FAKE_SECURITY_STATE/initial_default_mode")"
    case "$initial_mode" in
      exit37)
        return 37
        ;;
      exit44)
        return 44
        ;;
      error61)
        return 61
        ;;
      missing)
        cat "$FAKE_SECURITY_STATE/initial_default_output"
        return 0
        ;;
      ok)
        ;;
      *)
        echo "unknown fake default mode: $initial_mode" >&2
        return 92
        ;;
    esac
  fi

  current_default="$(cat "$FAKE_SECURITY_STATE/default_path")"
  current_keychain="$(cat "$FAKE_SECURITY_STATE/current_keychain")"
  if fake_is_current_keychain "$current_default"; then
    if fake_fail_once temp_default_readback; then
      return 73
    fi
  elif [ -n "$current_keychain" ]; then
    if fake_fail_once cleanup_default_readback; then
      return 73
    fi
  elif [ "$count" -gt 1 ] && fake_fail_once repair_default_readback; then
    return 73
  fi

  printf '    "%s"\n' "$current_default"
}

fake_security_default_set() {
  default_path="$5"
  if ! grep -F -x -- "$default_path" "$FAKE_SECURITY_STATE/search_list" >/dev/null; then
    echo "fake security rejected a default outside the active search list" >&2
    return 93
  fi

  default_kind=repair
  current_keychain="$(cat "$FAKE_SECURITY_STATE/current_keychain")"
  if fake_is_current_keychain "$default_path"; then
    default_kind=temp
  elif [ -n "$current_keychain" ]; then
    default_kind=cleanup
  fi

  if fake_fail_once "${default_kind}_default_set_before"; then
    return 73
  fi

  printf '%s\n' "$default_path" > "$FAKE_SECURITY_STATE/default_path"
  fake_log "default-set-$default_kind|$default_path"

  if fake_fail_once "${default_kind}_default_set_after"; then
    return 73
  fi
}

fake_security_create() {
  if [ "$#" -ne 4 ] || [ "$2" != "-p" ]; then
    return 94
  fi
  keychain_path="$4"
  if ! fake_is_managed_keychain "$keychain_path"; then
    echo "fake security rejected unmanaged current keychain: $keychain_path" >&2
    return 90
  fi
  if [ -e "$keychain_path" ]; then
    echo "fake security rejected reuse of an existing keychain: $keychain_path" >&2
    return 90
  fi
  printf '%s\n' "$keychain_path" > "$FAKE_SECURITY_STATE/current_keychain"
  : > "$keychain_path"
  fake_log "create-temp|$keychain_path"
}

fake_security_unlock() {
  if [ "$#" -ne 4 ] || [ "$2" != "-p" ]; then
    return 94
  fi
  fake_require_current_keychain "$4"
  [ -f "$4" ]
  fake_log "unlock-temp"
}

fake_security_settings() {
  if [ "$#" -ne 5 ] || [ "$2" != "-u" ] || [ "$3" != "-t" ] \
    || [ "$4" != "7200" ]; then
    return 94
  fi
  fake_require_current_keychain "$5"
  [ -f "$5" ]
  fake_log "settings-temp"
}

fake_security_add_item() {
  if [ "$#" -ne 8 ] || [ "$2" != "-a" ] || [ "$4" != "-s" ] \
    || [ "$6" != "-w" ]; then
    return 94
  fi
  fake_require_current_keychain "$8"
  [ "$(cat "$FAKE_SECURITY_STATE/default_path")" = "$8" ]
  : > "$FAKE_SECURITY_STATE/item_exists"
  fake_log "item-add-temp"
}

fake_security_find_item() {
  if { [ "$#" -ne 6 ] && [ "$#" -ne 7 ]; } \
    || [ "$2" != "-a" ] || [ "$4" != "-s" ] || [ "$6" != "-w" ]; then
    return 94
  fi

  if [ "$#" -eq 6 ]; then
    if [ "$(cat "$FAKE_SECURITY_STATE/implicit_search_mode")" = "stale" ]; then
      fake_log "item-find-implicit-miss"
      echo "security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain." >&2
      return 44
    fi
    keychain_path="$(cat "$FAKE_SECURITY_STATE/default_path")"
    log_entry="item-find-implicit"
  else
    keychain_path="$7"
    log_entry="item-find-explicit|$keychain_path"
  fi

  fake_require_current_keychain "$keychain_path"
  [ -e "$FAKE_SECURITY_STATE/item_exists" ]
  fake_log "$log_entry"
  printf '%s\n' "writable"
}

fake_security_delete_item() {
  if { [ "$#" -ne 5 ] && [ "$#" -ne 6 ]; } \
    || [ "$2" != "-a" ] || [ "$4" != "-s" ]; then
    return 94
  fi

  if [ "$#" -eq 5 ]; then
    keychain_path="$(cat "$FAKE_SECURITY_STATE/default_path")"
    log_entry="item-delete-implicit"
  else
    keychain_path="$6"
    log_entry="item-delete-explicit|$keychain_path"
  fi

  fake_require_current_keychain "$keychain_path"
  [ -e "$FAKE_SECURITY_STATE/item_exists" ]
  /bin/rm -f "$FAKE_SECURITY_STATE/item_exists"
  fake_log "$log_entry"
}

fake_security_delete_keychain() {
  if [ "$#" -ne 2 ]; then
    return 94
  fi
  fake_require_current_keychain "$2"
  /bin/rm -f "$2"
  fake_log "delete-temp"
}

fake_security() {
  if [ "$#" -eq 3 ] && [ "$1" = "list-keychains" ] \
    && [ "$2" = "-d" ] && [ "$3" = "user" ]; then
    fake_security_list_query
    return
  fi
  if [ "$#" -ge 5 ] && [ "$1" = "list-keychains" ] \
    && [ "$2" = "-d" ] && [ "$3" = "user" ] && [ "$4" = "-s" ]; then
    fake_security_list_set "$@"
    return
  fi
  if [ "$#" -eq 3 ] && [ "$1" = "default-keychain" ] \
    && [ "$2" = "-d" ] && [ "$3" = "user" ]; then
    fake_security_default_query
    return
  fi
  if [ "$#" -eq 5 ] && [ "$1" = "default-keychain" ] \
    && [ "$2" = "-d" ] && [ "$3" = "user" ] && [ "$4" = "-s" ]; then
    fake_security_default_set "$@"
    return
  fi

  case "${1:-}" in
    create-keychain)
      fake_security_create "$@"
      ;;
    unlock-keychain)
      fake_security_unlock "$@"
      ;;
    set-keychain-settings)
      fake_security_settings "$@"
      ;;
    add-generic-password)
      fake_security_add_item "$@"
      ;;
    find-generic-password)
      fake_security_find_item "$@"
      ;;
    delete-generic-password)
      fake_security_delete_item "$@"
      ;;
    delete-keychain)
      fake_security_delete_keychain "$@"
      ;;
    *)
      echo "fake security rejected argv shape" >&2
      return 94
      ;;
  esac
}

if [ "$(basename "$0")" = "security" ]; then
  fake_security "$@"
  exit $?
fi

if [ "$(basename "$0")" = "xcrun" ]; then
  body_mode="$(cat "$FAKE_SECURITY_STATE/body_mode")"
  case "$body_mode" in
    no_simulator)
      exit 0
      ;;
    success|sentinel23)
      printf '%s\n' "    iPhone 16 (00000000-0000-0000-0000-000000000000) (Booted)"
      exit 0
      ;;
    *)
      echo "unknown fake body mode: $body_mode" >&2
      exit 92
      ;;
  esac
fi

if [ "$(basename "$0")" = "bash" ]; then
  fake_log "body-build"
  exit 0
fi

if [ "$(basename "$0")" = "swift" ]; then
  if [ "$#" -ne 2 ] || [ "$1" != "test" ] || [ "$2" != "--no-parallel" ]; then
    exit 94
  fi
  fake_log "body-swift"
  if [ "$(cat "$FAKE_SECURITY_STATE/body_mode")" = "sentinel23" ]; then
    exit 23
  fi
  exit 0
fi

if [ "$(basename "$0")" = "xcodebuild" ]; then
  fake_log "body-xcodebuild"
  exit 0
fi

TEST_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
SWIFT_SDK_DIR="$(cd "$TEST_SCRIPT_DIR/.." && pwd -P)"
RUN_TESTS_SCRIPT="$SWIFT_SDK_DIR/run_tests.sh"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/swift-keychain-regression.XXXXXX")"
TEST_ROOT="$(cd "$TEST_ROOT" && pwd -P)"

cleanup_test_root() {
  original_status=$?
  trap - EXIT
  /bin/rm -rf "$TEST_ROOT"
  exit "$original_status"
}
trap cleanup_test_root EXIT

fail_test() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_equal() {
  expected="$1"
  actual="$2"
  message="$3"
  if [ "$actual" != "$expected" ]; then
    fail_test "$message (expected '$expected', got '$actual')"
  fi
}

assert_no_mutation() {
  state_dir="$1"
  if [ -s "$state_dir/command.log" ]; then
    fail_test "expected no Security mutation, got: $(cat "$state_dir/command.log")"
  fi
}

write_search_list() {
  output_file="$1"
  shift
  : > "$output_file"
  for keychain_path in "$@"; do
    printf '%s\n' "$keychain_path" >> "$output_file"
  done
}

assert_search_list() {
  state_dir="$1"
  shift
  expected_file="$state_dir/expected-search-list"
  write_search_list "$expected_file" "$@"
  if ! diff -u "$expected_file" "$state_dir/search_list"; then
    fail_test "user search list was not restored as expected"
  fi
}

new_case() {
  case_name="$1"
  case_dir="$TEST_ROOT/$case_name"
  case_home="$case_dir/home"
  case_keychains="$case_home/Library/Keychains"
  case_runner_temp="$case_dir/runner-temp"
  case_state="$case_dir/state"
  case_bin="$case_dir/bin"
  mkdir -p "$case_keychains" "$case_runner_temp" "$case_state" "$case_bin"
  for fake_command in security xcrun bash swift xcodebuild; do
    /bin/cp \
      "$TEST_SCRIPT_DIR/run-tests-keychain-regression.sh" \
      "$case_bin/$fake_command"
    chmod 700 "$case_bin/$fake_command"
  done

  printf '%s\n' "$case_runner_temp" > "$case_state/runner_temp"
  case_managed_root="$case_keychains/dash-ci-tests"
  printf '%s\n' "$case_managed_root" > "$case_state/managed_root"
  : > "$case_state/current_keychain"
  printf '%s\n' "ok" > "$case_state/initial_list_mode"
  : > "$case_state/initial_list_output"
  printf '%s\n' "ok" > "$case_state/initial_default_mode"
  : > "$case_state/initial_default_output"
  printf '%s\n' "no_simulator" > "$case_state/body_mode"
  printf '%s\n' "current" > "$case_state/implicit_search_mode"
  printf '%s\n' "1" > "$case_state/repair_opt_in"
  : > "$case_state/search_list"
  : > "$case_state/default_path"
  : > "$case_state/fail_at"
  : > "$case_state/command.log"
  printf '%s\n' "0" > "$case_state/default_query_count"
  printf '%s\n' "0" > "$case_state/list_query_count"

  case_login="$case_keychains/login.keychain-db"
  case_second="$case_keychains/second keychain.keychain-db"
  case_dead="$case_keychains/dead.keychain-db"
  case_outside="$case_dir/outside.keychain-db"
  : > "$case_login"
  : > "$case_second"
  : > "$case_outside"
  chmod 600 "$case_login" "$case_second" "$case_outside"
}

execute_case() {
  : > "$case_state/current_keychain"
  /bin/rm -f "$case_state/failure_used"
  printf '%s\n' "0" > "$case_state/default_query_count"
  printf '%s\n' "0" > "$case_state/list_query_count"
  set +e
  CI=1 \
  GITHUB_ACTIONS=true \
  DASH_SWIFT_CI_ALLOW_UNREADABLE_KEYCHAIN_REPAIR="$(
    cat "$case_state/repair_opt_in"
  )" \
  HOME="$case_home" \
  RUNNER_TEMP="$case_runner_temp" \
  FAKE_SECURITY_STATE="$case_state" \
  PATH="$case_bin:/usr/bin:/bin:/usr/sbin:/sbin" \
    /bin/bash "$RUN_TESTS_SCRIPT" > "$case_dir/stdout" 2> "$case_dir/stderr"
  case_status=$?
  set -e
}

run_case() {
  expected_status="$1"
  execute_case
  if [ "$case_status" -ne "$expected_status" ]; then
    cat "$case_dir/stderr" >&2
    fail_test "$case_name exit status (expected '$expected_status', got '$case_status')"
  fi
}

assert_no_temporary_artifacts() {
  if [ -d "$case_managed_root" ] \
    && find "$case_managed_root" -mindepth 1 -print -quit | grep -q .; then
    fail_test "$case_name leaked temporary keychain artifacts"
  fi
}

assert_cleanup_order() {
  actual_order="$(grep -E '^(default-set-cleanup|list-set-cleanup|delete-temp)' \
    "$case_state/command.log" | tail -3 | sed -E 's/\|.*$//' | tr '\n' ' ')"
  assert_equal "default-set-cleanup list-set-cleanup delete-temp " \
    "$actual_order" "$case_name cleanup order"
}

assert_default_is_contained() {
  selected_default="$(cat "$case_state/default_path")"
  if [ ! -f "$selected_default" ]; then
    fail_test "$case_name selected default does not exist: $selected_default"
  fi
  if ! grep -F -x -- "$selected_default" "$case_state/search_list" >/dev/null; then
    fail_test "$case_name selected default is outside the active search list"
  fi
}

assert_current_keychain_retained() {
  current_keychain="$(cat "$case_state/current_keychain")"
  if [ -z "$current_keychain" ] || [ ! -f "$current_keychain" ]; then
    fail_test "$case_name did not retain the current managed keychain"
  fi
}

new_case stale_implicit_search_uses_explicit_smoke_keychain
write_search_list "$case_state/search_list" "$case_login"
printf '%s\n' "$case_login" > "$case_state/default_path"
printf '%s\n' "sentinel23" > "$case_state/body_mode"
printf '%s\n' "stale" > "$case_state/implicit_search_mode"
execute_case
case "$case_status" in
  44)
    if ! grep -F -x "item-add-temp" "$case_state/command.log" >/dev/null; then
      fail_test "$case_name did not add the smoke item to the temporary keychain"
    fi
    if ! grep -F -x "item-find-implicit-miss" "$case_state/command.log" >/dev/null; then
      fail_test "$case_name did not reproduce the stale implicit lookup"
    fi
    if grep -E '^(item-delete-|body-)' "$case_state/command.log" >/dev/null; then
      fail_test "$case_name reached delete or the test body after the implicit lookup failed"
    fi
    if ! grep -F -x \
      "security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain." \
      "$case_dir/stderr" >/dev/null; then
      fail_test "$case_name did not reproduce the runner's exact Security diagnostic"
    fi
    ;;
  23)
    smoke_keychain="$(cat "$case_state/current_keychain")"
    if ! grep -F -x "item-find-explicit|$smoke_keychain" \
      "$case_state/command.log" >/dev/null; then
      fail_test "$case_name did not read the smoke item from the explicit temporary keychain"
    fi
    if ! grep -F -x "item-delete-explicit|$smoke_keychain" \
      "$case_state/command.log" >/dev/null; then
      fail_test "$case_name did not delete the smoke item from the explicit temporary keychain"
    fi
    if ! grep -F -x "body-swift" "$case_state/command.log" >/dev/null; then
      fail_test "$case_name did not reach the serialized Swift test body"
    fi
    if grep -F -x "item-find-implicit-miss" "$case_state/command.log" >/dev/null; then
      fail_test "$case_name still used the stale implicit lookup"
    fi
    ;;
  *)
    cat "$case_dir/stderr" >&2
    fail_test "$case_name unexpected exit status '$case_status'"
    ;;
esac
assert_equal "23" "$case_status" "$case_name primary status"
assert_no_temporary_artifacts
assert_cleanup_order

new_case exit44_list_prefers_login
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_dead" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_list_mode"
printf '    "%s"\n' "$case_second" > "$case_state/initial_list_output"
printf '%s\n' "exit44" > "$case_state/initial_default_mode"
run_case 1
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "exit-44 list recovery default"
assert_search_list "$case_state" "$case_login"
assert_no_temporary_artifacts
assert_cleanup_order

new_case exit37_list_prefers_login
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_dead" > "$case_state/default_path"
printf '%s\n' "exit37" > "$case_state/initial_list_mode"
printf '    "%s"\n' "$case_second" > "$case_state/initial_list_output"
printf '%s\n' "exit37" > "$case_state/initial_default_mode"
run_case 1
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "exit-37 list recovery default"
assert_search_list "$case_state" "$case_login"
assert_no_temporary_artifacts

new_case unreadable_list_preserves_valid_default
write_search_list "$case_state/search_list" "$case_dead"
printf '%s\n' "$case_second" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_list_mode"
printf '    "%s"\n' "$case_dead" > "$case_state/initial_list_output"
run_case 1
assert_equal "$case_second" "$(cat "$case_state/default_path")" \
  "unreadable-list valid default"
assert_search_list "$case_state" "$case_second" "$case_login"
assert_no_temporary_artifacts

new_case unreadable_list_rejects_outside_default
write_search_list "$case_state/search_list" "$case_dead"
printf '%s\n' "$case_outside" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_list_mode"
run_case 1
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "unreadable-list outside default"
assert_search_list "$case_state" "$case_login"
assert_no_temporary_artifacts

new_case unreadable_list_rejects_symlink_default
case_symlink="$case_keychains/symlink.keychain-db"
ln -s "$case_second" "$case_symlink"
write_search_list "$case_state/search_list" "$case_dead"
printf '%s\n' "$case_symlink" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_list_mode"
run_case 1
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "unreadable-list symlink default"
assert_search_list "$case_state" "$case_login"
assert_no_temporary_artifacts

new_case unreadable_list_rejects_unwritable_default
chmod 400 "$case_second"
write_search_list "$case_state/search_list" "$case_dead"
printf '%s\n' "$case_second" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_list_mode"
run_case 1
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "unreadable-list unwritable default"
assert_search_list "$case_state" "$case_login"
assert_no_temporary_artifacts

new_case unreadable_list_without_fallback
/bin/rm -f "$case_login"
write_search_list "$case_state/search_list" "$case_dead"
printf '%s\n' "$case_outside" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_list_mode"
run_case 1
assert_no_mutation "$case_state"
assert_no_temporary_artifacts

new_case unreadable_list_requires_opt_in
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_dead" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_list_mode"
printf '%s\n' "exit44" > "$case_state/initial_default_mode"
: > "$case_state/repair_opt_in"
run_case 1
assert_no_mutation "$case_state"
assert_no_temporary_artifacts

new_case unrelated_list_error
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_second" > "$case_state/default_path"
printf '%s\n' "error61" > "$case_state/initial_list_mode"
run_case 61
assert_no_mutation "$case_state"
assert_no_temporary_artifacts

new_case namespace_symlink
namespace_target="$case_dir/namespace-target"
mkdir -m 700 "$namespace_target"
ln -s "$namespace_target" "$case_managed_root"
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_second" > "$case_state/default_path"
run_case 1
assert_no_mutation "$case_state"
if find "$namespace_target" -mindepth 1 -print -quit | grep -q .; then
  fail_test "$case_name created a child through the managed namespace symlink"
fi

new_case namespace_regular_file
: > "$case_managed_root"
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_second" > "$case_state/default_path"
run_case 1
assert_no_mutation "$case_state"

new_case namespace_wrong_mode
mkdir -m 755 "$case_managed_root"
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_second" > "$case_state/default_path"
run_case 1
assert_no_mutation "$case_state"
assert_equal "755" "$(stat -f '%Lp' "$case_managed_root")" \
  "managed namespace mode"

new_case exit44_prefers_login
write_search_list "$case_state/search_list" \
  "$case_second" "$case_keychains/./second keychain.keychain-db" "$case_dead"
printf '%s\n' "$case_dead" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_default_mode"
run_case 1
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "exit-44 recovery default"
assert_search_list "$case_state" "$case_login" "$case_second"
assert_no_temporary_artifacts
assert_cleanup_order
if grep -F "writable" "$case_state/command.log" >/dev/null; then
  fail_test "fake Security command log recorded the smoke-test value"
fi

new_case coherent_nonfirst_default
mkdir -m 700 "$case_managed_root"
write_search_list "$case_state/search_list" "$case_second" "$case_login" "$case_dead"
printf '%s\n' "$case_login" > "$case_state/default_path"
run_case 1
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "coherent default restoration"
assert_search_list "$case_state" "$case_second" "$case_login" "$case_dead"
assert_no_temporary_artifacts
assert_cleanup_order
assert_equal "700" "$(stat -f '%Lp' "$case_managed_root")" \
  "managed namespace mode"

new_case coherent_outside_default
write_search_list "$case_state/search_list" "$case_outside"
printf '%s\n' "$case_outside" > "$case_state/default_path"
run_case 1
assert_equal "$case_outside" "$(cat "$case_state/default_path")" \
  "outside coherent default restoration"
assert_search_list "$case_state" "$case_outside"
assert_no_temporary_artifacts
assert_cleanup_order

new_case coherent_baseline_filters_all_managed_entries
mkdir -m 700 "$case_managed_root"
existing_managed_dir="$case_managed_root/run.existing"
existing_managed="$existing_managed_dir/tests.keychain-db"
missing_managed="$case_managed_root/run.missing/tests.keychain-db"
symlink_managed_dir="$case_managed_root/run.symlink"
symlink_managed="$symlink_managed_dir/tests.keychain-db"
outside_alias="$case_managed_root/../login.keychain-db"
mkdir -m 700 "$existing_managed_dir" "$symlink_managed_dir"
: > "$existing_managed"
chmod 600 "$existing_managed"
ln -s "$case_login" "$symlink_managed"
write_search_list \
  "$case_state/search_list" \
  "$case_login" \
  "$existing_managed" \
  "$missing_managed" \
  "$symlink_managed" \
  "$outside_alias"
printf '%s\n' "$case_login" > "$case_state/default_path"
run_case 1
current_keychain="$(cat "$case_state/current_keychain")"
if [ -e "$current_keychain" ]; then
  fail_test "$case_name did not remove the current managed keychain"
fi
if [ ! -f "$existing_managed" ] || [ ! -L "$symlink_managed" ]; then
  fail_test "$case_name changed a protected managed entry"
fi
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "managed-filter coherent default"
assert_search_list "$case_state" "$case_login" "$outside_alias"

new_case exit_zero_missing_default
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_dead" > "$case_state/default_path"
printf '%s\n' "missing" > "$case_state/initial_default_mode"
printf '    "%s"\n' "$case_dead" > "$case_state/initial_default_output"
run_case 1
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "missing-path recovery default"
assert_search_list "$case_state" "$case_login" "$case_second"
assert_no_temporary_artifacts

new_case first_eligible_without_login
/bin/rm -f "$case_login"
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_dead" > "$case_state/default_path"
printf '%s\n' "exit37" > "$case_state/initial_default_mode"
run_case 1
assert_equal "$case_second" "$(cat "$case_state/default_path")" \
  "first eligible recovery default"
assert_search_list "$case_state" "$case_second"
assert_no_temporary_artifacts

new_case no_eligible_fallback
/bin/rm -f "$case_login" "$case_second"
write_search_list "$case_state/search_list" "$case_dead" "$case_outside"
printf '%s\n' "$case_dead" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_default_mode"
run_case 1
assert_no_mutation "$case_state"
assert_no_temporary_artifacts

new_case empty_raw_list
printf '%s\n' "$case_dead" > "$case_state/default_path"
printf '%s\n' "exit44" > "$case_state/initial_default_mode"
run_case 1
assert_no_mutation "$case_state"
assert_no_temporary_artifacts

new_case unrelated_default_error
write_search_list "$case_state/search_list" "$case_second"
printf '%s\n' "$case_second" > "$case_state/default_path"
printf '%s\n' "error61" > "$case_state/initial_default_mode"
run_case 61
assert_no_mutation "$case_state"
assert_no_temporary_artifacts

if ! grep -F -- '[ ! -O "$canonical_path" ]' "$RUN_TESTS_SCRIPT" >/dev/null; then
  fail_test "run_tests.sh no longer rejects recovery keychains not owned by the user"
fi

for injected_failure in \
  repair_list_set_before \
  repair_list_set_after \
  repair_default_set_before \
  repair_default_set_after \
  repair_default_readback \
  repair_list_readback \
  temp_list_set_before \
  temp_list_set_after \
  temp_default_set_before \
  temp_default_set_after \
  temp_default_readback
do
  new_case "failure_$injected_failure"
  write_search_list "$case_state/search_list" "$case_second"
  printf '%s\n' "$case_dead" > "$case_state/default_path"
  printf '%s\n' "exit44" > "$case_state/initial_default_mode"
  printf '%s\n' "$injected_failure" > "$case_state/fail_at"
  run_case 73

  case "$injected_failure" in
    repair_list_set_before|repair_list_set_after)
      assert_search_list "$case_state" "$case_second"
      assert_equal "$case_dead" "$(cat "$case_state/default_path")" \
        "$case_name unchanged default"
      ;;
    repair_default_set_before)
      assert_search_list "$case_state" "$case_login" "$case_second"
      assert_equal "$case_dead" "$(cat "$case_state/default_path")" \
        "$case_name pre-mutation default"
      ;;
    *)
      assert_search_list "$case_state" "$case_login" "$case_second"
      assert_equal "$case_login" "$(cat "$case_state/default_path")" \
        "$case_name contained default"
      ;;
  esac
  case "$injected_failure" in
    repair_*)
      if grep -F "create-temp" "$case_state/command.log" >/dev/null; then
        fail_test "$case_name created a temporary keychain before repair committed"
      fi
      ;;
  esac
  assert_no_temporary_artifacts
done

for injected_failure in \
  repair_list_set_before \
  repair_list_set_after
do
  new_case "unreadable_list_$injected_failure"
  write_search_list "$case_state/search_list" "$case_second"
  printf '%s\n' "$case_dead" > "$case_state/default_path"
  printf '%s\n' "exit44" > "$case_state/initial_list_mode"
  printf '    "%s"\n' "$case_second" > "$case_state/initial_list_output"
  printf '%s\n' "exit44" > "$case_state/initial_default_mode"
  printf '%s\n' "$injected_failure" > "$case_state/fail_at"
  run_case 73

  if [ "$injected_failure" = "repair_list_set_before" ]; then
    assert_search_list "$case_state" "$case_second"
  else
    assert_search_list "$case_state" "$case_login"
  fi
  assert_equal "$case_dead" "$(cat "$case_state/default_path")" \
    "$case_name unchanged default"
  if grep -F "create-temp" "$case_state/command.log" >/dev/null; then
    fail_test "$case_name created a temporary keychain before repair committed"
  fi
  assert_no_temporary_artifacts
done

for injected_failure in \
  cleanup_default_set_before \
  cleanup_default_set_after \
  cleanup_default_readback \
  cleanup_list_set_before \
  cleanup_list_set_after \
  cleanup_list_readback
do
  new_case "failure_$injected_failure"
  write_search_list "$case_state/search_list" "$case_login"
  printf '%s\n' "$case_login" > "$case_state/default_path"
  printf '%s\n' "success" > "$case_state/body_mode"
  printf '%s\n' "$injected_failure" > "$case_state/fail_at"
  run_case 1
  assert_default_is_contained

  case "$injected_failure" in
    cleanup_default_set_before|cleanup_default_readback|cleanup_list_set_before|cleanup_list_readback)
      assert_current_keychain_retained
      ;;
    *)
      assert_no_temporary_artifacts
      ;;
  esac
done

new_case cleanup_preserves_primary_status
write_search_list "$case_state/search_list" "$case_login"
printf '%s\n' "$case_login" > "$case_state/default_path"
printf '%s\n' "sentinel23" > "$case_state/body_mode"
printf '%s\n' "cleanup_default_set_before" > "$case_state/fail_at"
run_case 23
assert_default_is_contained
assert_current_keychain_retained

new_case retained_orphans_are_recovered_not_reused
write_search_list "$case_state/search_list" "$case_login"
printf '%s\n' "$case_login" > "$case_state/default_path"
printf '%s\n' "success" > "$case_state/body_mode"
printf '%s\n' "cleanup_default_set_before" > "$case_state/fail_at"
run_case 1
assert_default_is_contained
assert_current_keychain_retained
first_orphan="$(cat "$case_state/current_keychain")"
first_orphan_dir="$(dirname "$first_orphan")"
assert_equal "700" "$(stat -f '%Lp' "$first_orphan_dir")" \
  "retained orphan directory mode"

/bin/rm -rf "$case_runner_temp"
mkdir -m 700 "$case_runner_temp"
assert_default_is_contained
if [ ! -f "$first_orphan" ]; then
  fail_test "$case_name lost retained containment after runner temp purge"
fi

seeded_orphan_dir="$case_managed_root/run.seeded-orphan"
seeded_orphan="$seeded_orphan_dir/tests.keychain-db"
mkdir -m 700 "$seeded_orphan_dir"
: > "$seeded_orphan"
chmod 600 "$seeded_orphan"
write_search_list \
  "$case_state/search_list" "$first_orphan" "$seeded_orphan" "$case_login"
printf '%s\n' "$first_orphan" > "$case_state/default_path"
: > "$case_state/fail_at"
printf '%s\n' "ok" > "$case_state/initial_list_mode"
printf '%s\n' "ok" > "$case_state/initial_default_mode"
run_case 0
second_run_keychain="$(cat "$case_state/current_keychain")"

if [ "$second_run_keychain" = "$first_orphan" ] \
  || [ "$second_run_keychain" = "$seeded_orphan" ]; then
  fail_test "$case_name reused a retained managed keychain"
fi
if [ -e "$second_run_keychain" ]; then
  fail_test "$case_name did not remove the second run keychain"
fi
if [ ! -f "$first_orphan" ] || [ ! -f "$seeded_orphan" ]; then
  fail_test "$case_name deleted a protected managed orphan"
fi
assert_equal "700" "$(stat -f '%Lp' "$first_orphan_dir")" \
  "retained orphan directory mode after recovery"
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "second-run recovery default"
assert_search_list "$case_state" "$case_login"
if grep -F "$case_managed_root/" "$case_state/search_list" >/dev/null; then
  fail_test "$case_name retained a managed path in the final search list"
fi

echo "Swift CI keychain regression passed"
