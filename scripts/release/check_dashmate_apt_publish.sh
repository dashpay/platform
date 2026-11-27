#!/usr/bin/env bash

set -euo pipefail

cmd_usage="Usage: scripts/release/check_dashmate_apt_publish.sh --tag=<tag> --suite=<suite> --sha256sums-sha256=<sha256> [--repo=OWNER/NAME]

Checks that an apt suite of the dashmate repository serves exactly the debs a release run verified.
The release's SHA256SUMS must still have the sha256 that the run's Release Dashmate checksums job
logged when it attached the file. Then a throwaway apt client in Docker, which verifies the suite's
signature against the archive keyring, must find the sha256 that SHA256SUMS lists for the amd64
deb and for the arm64 deb, and each deb the suite's index names is downloaded from the repository
and must have that sha256.

  Options:
  --tag=TAG                  - release tag, e.g. v5.1.0
  --suite=SUITE              - dev, testing or stable
  --sha256sums-sha256=SHA256 - the sha256 in the checksums job's line
                               'SHA256SUMS attached: asset <id> sha256 <sha256>'
  --repo=OWNER/NAME          - repository, default dashpay/platform
  -h          --help         - show help
"

# The archive keyring as infra published it on dashpay/platform#4246, by its sha256.
KEYRING_URL=https://apt.services.dash.org/dashmate/dashmate-archive-keyring.gpg
KEYRING_SHA256=08006df56f24e282f19012a29e539dad431654abefefd697832ac61918b01230
REPOSITORY_URL=https://apt.services.dash.org/dashmate
IMAGE=ubuntu:24.04

fail() {
  echo "Error: $*" >&2
  exit 1
}

usage_error() {
  echo "Error: $*" >&2
  echo "$cmd_usage" >&2
  exit 2
}

TAG=""
SUITE=""
DIGEST=""
REPO="dashpay/platform"

for i in "$@"
do
  case ${i} in
    -h|--help)
      echo "$cmd_usage"
      exit 0
      ;;
    --tag=*)
      TAG="${i#*=}"
      ;;
    --suite=*)
      SUITE="${i#*=}"
      ;;
    --sha256sums-sha256=*)
      DIGEST="${i#*=}"
      ;;
    --repo=*)
      REPO="${i#*=}"
      ;;
    *)
      usage_error "unknown option ${i}"
      ;;
  esac
done

tag_pattern='^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$'
[[ "${TAG}" =~ ${tag_pattern} ]] || usage_error "--tag must be a release tag such as v5.1.0, got '${TAG}'"
case "${SUITE}" in
  dev | testing | stable) ;;
  *) usage_error "--suite must be dev, testing or stable, got '${SUITE}'" ;;
esac
DIGEST="$(echo "${DIGEST}" | tr 'A-F' 'a-f')"
digest_pattern='^[0-9a-f]{64}$'
[[ "${DIGEST}" =~ ${digest_pattern} ]] || usage_error "--sha256sums-sha256 must be a sha256, got '${DIGEST}'"

for tool in gh docker curl; do
  command -v "${tool}" > /dev/null 2>&1 || fail "${tool} is not installed"
done

# GNU sha256sum, or shasum where it is missing.
if sha256sum --version 2> /dev/null | grep -q 'GNU coreutils'; then
  sha256=(sha256sum)
else
  sha256=(shasum -a 256)
fi

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/dashmate-apt-check.XXXXXX")"
trap 'rm -rf "${WORK_DIR}"' EXIT

gh release download "${TAG}" --repo "${REPO}" --pattern SHA256SUMS --dir "${WORK_DIR}" \
  || fail "could not download the SHA256SUMS of ${TAG}"
sum="$("${sha256[@]}" "${WORK_DIR}/SHA256SUMS" | cut -d' ' -f1)"
[ "${sum}" = "${DIGEST}" ] \
  || fail "the SHA256SUMS of ${TAG} has sha256 ${sum}, which is not the one the Release Dashmate checksums job verified (${DIGEST}); the release is not done, report it to infra"

# The sha256 SHA256SUMS lists for the one deb of an architecture.
expected() {
  local lines
  lines="$(awk -v suffix="_$1.deb" 'substr($2, length($2) - length(suffix) + 1) == suffix && $2 ~ /^dashmate_/' \
    "${WORK_DIR}/SHA256SUMS")"
  if [ -z "${lines}" ] || [ "$(echo "${lines}" | wc -l | tr -d ' ')" != 1 ]; then
    fail "the SHA256SUMS of ${TAG} does not list exactly one $1 deb"
  fi
  echo "${lines}" | cut -d' ' -f1
}
AMD64="$(expected amd64)"
ARM64="$(expected arm64)"

# The apt client prints the suite's Packages indexes for both architectures, as apt verified them.
# apt-get update fails on a bad or expired signature, and so does the check. apt only warns when a
# signed Release names another suite than the one asked for, so the client checks that itself.
SERVED="$(docker run --rm -e SUITE="${SUITE}" -e KEYRING_URL="${KEYRING_URL}" -e KEYRING_SHA256="${KEYRING_SHA256}" \
  -e REPOSITORY_URL="${REPOSITORY_URL}" "${IMAGE}" bash -euo pipefail -c '
    apt-get update -qq
    apt-get install -qq -y curl ca-certificates > /dev/null
    rm -f /etc/apt/sources.list.d/ubuntu.sources /etc/apt/sources.list
    keyring=/usr/share/keyrings/dashmate-archive-keyring.gpg
    curl -fsSo "$keyring" "$KEYRING_URL"
    echo "$KEYRING_SHA256  $keyring" | sha256sum --check --quiet
    for arch in amd64 arm64; do
      [ "$arch" = "$(dpkg --print-architecture)" ] || dpkg --add-architecture "$arch"
    done
    echo "deb [arch=amd64,arm64 signed-by=$keyring] $REPOSITORY_URL $SUITE main" > /etc/apt/sources.list.d/dashmate.list
    apt-get update -qq -o APT::Update::Error-Mode=any
    release=$(ls /var/lib/apt/lists/*_dists_"$SUITE"_InRelease)
    suite=$(sed -n "s/^Suite: //p" "$release")
    codename=$(sed -n "s/^Codename: //p" "$release")
    if [ "$suite" != "$SUITE" ] || [ "$codename" != "$SUITE" ]; then
      echo "the signed Release names suite ${suite:-none} and codename ${codename:-none}, not $SUITE" >&2
      exit 3
    fi
    for arch in amd64 arm64; do
      index=$(apt-get indextargets --format "\$(FILENAME)" "Created-By: Packages" "Architecture: $arch")
      if [ -z "$index" ] || [ "$(echo "$index" | wc -l)" != 1 ]; then
        echo "expected one $arch Packages index, found: ${index:-none}" >&2
        exit 3
      fi
      echo "### index $arch"
      /usr/lib/apt/apt-helper cat-file "$index"
      echo
    done
  ')" || fail "the apt client could not read the ${SUITE} suite; the release is not done, report it to infra"

# One tab-separated `<architecture> <version> <sha256> <size> <file name>` line per dashmate record
# that its own architecture's index lists. The size and the file name are whole fields, spaces
# included, so the checks below see all of them.
RECORDS="$(printf '%s\n' "${SERVED}" | awk 'BEGIN {OFS = "\t"}
  function emit() {
    if (package == "dashmate" && arch != "" && sha != "" && ("x" arch) == ("x" index_arch)) print arch, version, sha, size, filename
    package = arch = version = sha = size = filename = ""
  }
  /^### index / {emit(); index_arch = $3; next}
  /^Package: / {package = $2}
  /^Architecture: / {arch = $2}
  /^Version: / {version = $2}
  /^SHA256: / {sha = $2}
  /^Size: / {size = substr($0, 7)}
  /^Filename: / {filename = substr($0, 11)}
  /^$/ {emit()}
  END {emit()}')"

for pair in "amd64 ${AMD64}" "arm64 ${ARM64}"; do
  arch="${pair%% *}"
  want="${pair#* }"
  # Compared as strings: awk would compare two all-digit sha256s as numbers.
  found="$(printf '%s\n' "${RECORDS}" | awk -F '\t' -v arch="${arch}" -v sha="${want}" '("x" $1) == ("x" arch) && ("x" $3) == ("x" sha)')"
  [ -n "${found}" ] \
    || fail "${SUITE} does not serve the ${arch} deb that SHA256SUMS lists (${want}); it serves: $(printf '%s\n' "${RECORDS}" | awk -v arch="${arch}" '("x" $1) == ("x" arch) {print $2 " " $3}' | paste -sd ',' -); the release is not done, report it to infra"
  # The index only lists the deb: every file a matching record names must be there, with those bytes.
  # Split by hand: read with a tab in IFS would strip tabs around the file name.
  while IFS= read -r record; do
    version="${record#*$'\t'}"
    filename="${version#*$'\t'}"
    filename="${filename#*$'\t'}"
    size="${filename%%$'\t'*}"
    filename="${filename#*$'\t'}"
    version="${version%%$'\t'*}"
    # A positive number of bytes, short enough for shell arithmetic.
    case "${size}" in
      '' | 0* | *[!0-9]* | ?????????????*)
        fail "the ${arch} record in ${SUITE} lists size '${size}', not a number of bytes; the release is not done, report it to infra" ;;
    esac
    case "${filename}" in
      pool/*.deb) ;;
      *) fail "the ${arch} record in ${SUITE} names '${filename}', not a deb in the pool; the release is not done, report it to infra" ;;
    esac
    case "${filename}" in
      *..* | *[!A-Za-z0-9._+~/-]*) fail "the ${arch} record in ${SUITE} names '${filename}', not a plain pool path; the release is not done, report it to infra" ;;
    esac
    # Capped a little above the listed size, so an oversized response cannot fill the disk.
    curl -fsSL --proto '=https' --max-time 600 --max-filesize "$((size + 4096))" -o "${WORK_DIR}/${arch}.deb" \
      "${REPOSITORY_URL}/${filename}" \
      || fail "could not download the ${arch} deb ${filename} that ${SUITE} lists; the release is not done, report it to infra"
    # apt refuses a deb whose size is not the one its record lists.
    served_size="$(wc -c < "${WORK_DIR}/${arch}.deb" | tr -d ' ')"
    [ "${served_size}" = "${size}" ] \
      || fail "the ${arch} deb ${filename} in ${SUITE} has ${served_size} bytes, not the ${size} its record lists; the release is not done, report it to infra"
    served_sum="$("${sha256[@]}" "${WORK_DIR}/${arch}.deb" | cut -d' ' -f1)"
    [ "${served_sum}" = "${want}" ] \
      || fail "the ${arch} deb ${filename} in ${SUITE} has sha256 ${served_sum}, not the ${want} that SHA256SUMS lists; the release is not done, report it to infra"
    echo "${SUITE} serves the verified deb: ${arch} ${version} ${want}, downloaded and checked (${filename})"
  done <<< "${found}"
done
