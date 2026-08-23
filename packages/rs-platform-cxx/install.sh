#!/usr/bin/env bash

set -euo pipefail

if [[ "$#" -ne 1 ]]; then
    echo "usage: $0 PREFIX" >&2
    exit 1
fi

package_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
workspace_dir="$(cd "${package_dir}/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-${workspace_dir}/target/platform-cxx-standalone}"
profile="${CARGO_PROFILE:-release}"
artifact_dir="${target_dir}"
if [[ -n "${CARGO_BUILD_TARGET:-}" ]]; then
    artifact_dir="${artifact_dir}/${CARGO_BUILD_TARGET}"
fi
artifact_dir="${artifact_dir}/${profile}"
prefix="$1"

archive="${artifact_dir}/libdash_platform_cxx_bundle.a"
if [[ ! -f "${archive}" ]]; then
    echo "missing standalone archive: ${archive}" >&2
    exit 1
fi

shopt -s nullglob
bridge_headers=("${artifact_dir}"/build/dash-platform-cxx-*/out/cxxbridge/include/dash/platform/src/lib.rs.h)
runtime_headers=("${artifact_dir}"/build/dash-platform-cxx-*/out/cxxbridge/include/rust/cxx.h)
if [[ "${#bridge_headers[@]}" -ne 1 || "${#runtime_headers[@]}" -ne 1 ]]; then
    echo "expected exactly one generated CXX header set under ${artifact_dir}" >&2
    exit 1
fi

install -d \
    "${prefix}/include/dash/platform/src" \
    "${prefix}/include/rust" \
    "${prefix}/lib"
install -m 0644 "${package_dir}/ffi.h" "${prefix}/include/dash/platform/ffi.h"
install -m 0644 "${package_dir}/signer.h" "${prefix}/include/dash/platform/signer.h"
install -m 0644 "${bridge_headers[0]}" "${prefix}/include/dash/platform/src/lib.rs.h"
install -m 0644 "${runtime_headers[0]}" "${prefix}/include/rust/cxx.h"
install -m 0644 "${archive}" "${prefix}/lib/libdash_platform_cxx.a"
