#!/usr/bin/env bash

set -euo pipefail

package_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
workspace_dir="$(cd "${package_dir}/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-${workspace_dir}/target/platform-cxx-standalone}"
cxx="${CXX:-c++}"
manifest="${package_dir}/standalone/Cargo.toml"

CARGO_TARGET_DIR="${target_dir}" cargo build --manifest-path "${manifest}" --locked

stage_dir="$(mktemp -d "${TMPDIR:-/tmp}/dash-platform-cxx.XXXXXX")"
trap 'rm -rf "${stage_dir}"' EXIT

CARGO_PROFILE=debug CARGO_TARGET_DIR="${target_dir}" \
    "${package_dir}/install.sh" "${stage_dir}"

system_libs=(-lpthread -lm)
case "$(uname -s)" in
    Darwin) system_libs+=(-framework CoreFoundation) ;;
    Linux) system_libs+=(-ldl) ;;
esac

"${cxx}" -std=c++20 -I"${stage_dir}/include" \
    "${package_dir}/tests/cxx_smoke.cc" \
    "${stage_dir}/lib/libdash_platform_cxx.a" \
    "${system_libs[@]}" -o "${stage_dir}/cxx_smoke"
"${stage_dir}/cxx_smoke"
