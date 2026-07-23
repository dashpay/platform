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

fake_is_temporary_keychain() {
  candidate_path="$1"
  runner_temp="$(cat "$FAKE_SECURITY_STATE/runner_temp")"
  case "$candidate_path" in
    "$runner_temp"/dash-ci-keychain.*/tests.keychain-db)
      return 0
      ;;
  esac
  return 1
}

fake_require_temporary_keychain() {
  candidate_path="$1"
  if ! fake_is_temporary_keychain "$candidate_path"; then
    echo "fake security rejected mutation of seeded keychain: $candidate_path" >&2
    return 90
  fi
}

fake_security_list_query() {
  count="$(cat "$FAKE_SECURITY_STATE/list_query_count")"
  count=$((count + 1))
  printf '%s\n' "$count" > "$FAKE_SECURITY_STATE/list_query_count"

  if [ "$count" -gt 1 ] && fake_fail_once repair_list_readback; then
    return 73
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
  if fake_is_temporary_keychain "$first_path"; then
    list_kind=temp
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
  if fake_is_temporary_keychain "$current_default"; then
    if fake_fail_once temp_default_readback; then
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
  if fake_is_temporary_keychain "$default_path"; then
    default_kind=temp
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
  fake_require_temporary_keychain "$keychain_path"
  : > "$keychain_path"
  fake_log "create-temp"
}

fake_security_unlock() {
  if [ "$#" -ne 4 ] || [ "$2" != "-p" ]; then
    return 94
  fi
  fake_require_temporary_keychain "$4"
  [ -f "$4" ]
  fake_log "unlock-temp"
}

fake_security_settings() {
  if [ "$#" -ne 5 ] || [ "$2" != "-u" ] || [ "$3" != "-t" ] \
    || [ "$4" != "7200" ]; then
    return 94
  fi
  fake_require_temporary_keychain "$5"
  [ -f "$5" ]
  fake_log "settings-temp"
}

fake_security_add_item() {
  if [ "$#" -ne 8 ] || [ "$2" != "-a" ] || [ "$4" != "-s" ] \
    || [ "$6" != "-w" ]; then
    return 94
  fi
  fake_require_temporary_keychain "$8"
  [ "$(cat "$FAKE_SECURITY_STATE/default_path")" = "$8" ]
  : > "$FAKE_SECURITY_STATE/item_exists"
  fake_log "item-add-temp"
}

fake_security_find_item() {
  if [ "$#" -ne 6 ] || [ "$2" != "-a" ] || [ "$4" != "-s" ] \
    || [ "$6" != "-w" ]; then
    return 94
  fi
  current_default="$(cat "$FAKE_SECURITY_STATE/default_path")"
  fake_require_temporary_keychain "$current_default"
  [ -e "$FAKE_SECURITY_STATE/item_exists" ]
  fake_log "item-find-default"
  printf '%s\n' "writable"
}

fake_security_delete_item() {
  if [ "$#" -ne 5 ] || [ "$2" != "-a" ] || [ "$4" != "-s" ]; then
    return 94
  fi
  current_default="$(cat "$FAKE_SECURITY_STATE/default_path")"
  fake_require_temporary_keychain "$current_default"
  [ -e "$FAKE_SECURITY_STATE/item_exists" ]
  /bin/rm -f "$FAKE_SECURITY_STATE/item_exists"
  fake_log "item-delete-default"
}

fake_security_delete_keychain() {
  if [ "$#" -ne 2 ]; then
    return 94
  fi
  fake_require_temporary_keychain "$2"
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
  /bin/cp "$TEST_SCRIPT_DIR/run-tests-keychain-regression.sh" "$case_bin/security"
  /bin/cp "$TEST_SCRIPT_DIR/run-tests-keychain-regression.sh" "$case_bin/xcrun"
  chmod 700 "$case_bin/security" "$case_bin/xcrun"

  printf '%s\n' "$case_runner_temp" > "$case_state/runner_temp"
  printf '%s\n' "ok" > "$case_state/initial_default_mode"
  : > "$case_state/initial_default_output"
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

run_case() {
  expected_status="$1"
  set +e
  CI=1 \
  GITHUB_ACTIONS=true \
  HOME="$case_home" \
  RUNNER_TEMP="$case_runner_temp" \
  FAKE_SECURITY_STATE="$case_state" \
  PATH="$case_bin:/usr/bin:/bin:/usr/sbin:/sbin" \
    /bin/bash "$RUN_TESTS_SCRIPT" > "$case_dir/stdout" 2> "$case_dir/stderr"
  case_status=$?
  set -e
  if [ "$case_status" -ne "$expected_status" ]; then
    cat "$case_dir/stderr" >&2
    fail_test "$case_name exit status (expected '$expected_status', got '$case_status')"
  fi
}

assert_no_temporary_artifacts() {
  if find "$case_runner_temp" -mindepth 1 -print -quit | grep -q .; then
    fail_test "$case_name leaked temporary keychain artifacts"
  fi
}

assert_cleanup_order() {
  actual_order="$(grep -E '^(default-set-repair|list-set-repair|delete-temp)' \
    "$case_state/command.log" | tail -3 | sed -E 's/\|.*$//' | tr '\n' ' ')"
  assert_equal "default-set-repair list-set-repair delete-temp " \
    "$actual_order" "$case_name cleanup order"
}

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
write_search_list "$case_state/search_list" "$case_second" "$case_login" "$case_dead"
printf '%s\n' "$case_login" > "$case_state/default_path"
run_case 1
assert_equal "$case_login" "$(cat "$case_state/default_path")" \
  "coherent default restoration"
assert_search_list "$case_state" "$case_second" "$case_login" "$case_dead"
assert_no_temporary_artifacts
assert_cleanup_order

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

echo "Swift CI keychain regression passed"
