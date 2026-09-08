#!/usr/bin/env bash
# Builds the crate, then compiles and runs tests/cxx_smoke.cc against the
# staged headers and the static archive exactly as an embedder would.
set -euo pipefail

package_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workspace_dir="$(cd "${package_dir}/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-${workspace_dir}/target}"
profile="${PROFILE:-debug}"
cxx="${CXX:-c++}"

if [[ "${profile}" == "release" ]]; then
    cargo build -p dash-platform-cxx --locked --release
else
    cargo build -p dash-platform-cxx --locked
fi

artifact_dir="${target_dir}/${profile}"
system_libs=(-lpthread -lm)
case "$(uname -s)" in
    Darwin) system_libs+=(-framework CoreFoundation -framework Security) ;;
    Linux) system_libs+=(-ldl) ;;
esac

out="$(mktemp -d "${TMPDIR:-/tmp}/dash-platform-cxx.XXXXXX")"
trap 'rm -rf "${out}"' EXIT

"${cxx}" -std=c++20 -I"${artifact_dir}/include" \
    "${package_dir}/tests/cxx_smoke.cc" \
    "${artifact_dir}/libdash_platform_cxx.a" \
    "${system_libs[@]}" -o "${out}/cxx_smoke"
"${out}/cxx_smoke"
echo "cxx_smoke: ok"
