#!/usr/bin/env bash

set -euo pipefail

cmd_usage="Usage: scripts/release/check_dashmate_apt_publish.sh --tag=<tag> --suite=<suite> --sha256sums-sha256=<sha256> [--repo=OWNER/NAME]

Checks that an apt suite of the dashmate repository serves exactly the debs a release run verified.
The release's SHA256SUMS must still have the sha256 that the run's Release Dashmate checksums job
logged when it attached the file. Then a throwaway apt client in Docker, which verifies the suite's
signature against the archive keyring, must find the sha256 that SHA256SUMS lists for the amd64
deb and for the arm64 deb.

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

for tool in gh docker; do
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
  [ -n "${lines}" ] && [ "$(echo "${lines}" | wc -l | tr -d ' ')" = 1 ] \
    || fail "the SHA256SUMS of ${TAG} does not list exactly one $1 deb"
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

# One `<architecture> <version> <sha256>` line per dashmate record that its own architecture's
# index lists.
RECORDS="$(echo "${SERVED}" | awk '
  function emit() {
    if (package == "dashmate" && arch != "" && sha != "" && ("x" arch) == ("x" index_arch)) print arch, version, sha
    package = arch = version = sha = ""
  }
  /^### index / {emit(); index_arch = $3; next}
  /^Package: / {package = $2}
  /^Architecture: / {arch = $2}
  /^Version: / {version = $2}
  /^SHA256: / {sha = $2}
  /^$/ {emit()}
  END {emit()}')"

for pair in "amd64 ${AMD64}" "arm64 ${ARM64}"; do
  arch="${pair%% *}"
  want="${pair#* }"
  # Compared as strings: awk would compare two all-digit sha256s as numbers.
  found="$(echo "${RECORDS}" | awk -v arch="${arch}" -v sha="${want}" '("x" $1) == ("x" arch) && ("x" $3) == ("x" sha)')"
  [ -n "${found}" ] \
    || fail "${SUITE} does not serve the ${arch} deb that SHA256SUMS lists (${want}); it serves: $(echo "${RECORDS}" | awk -v arch="${arch}" '("x" $1) == ("x" arch) {print $2 " " $3}' | paste -sd ',' -); the release is not done, report it to infra"
  echo "${SUITE} serves the verified deb: ${found}"
done
