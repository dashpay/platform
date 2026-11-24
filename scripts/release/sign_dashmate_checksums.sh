#!/usr/bin/env bash

set -euo pipefail

cmd_usage="Usage: scripts/release/sign_dashmate_checksums.sh --tag=<tag> --key=<fingerprint> --commit=<sha> [--upload]

Signs the SHA256SUMS of a published release with a maintainer's OpenPGP key. It first checks
that the release workflow made the file from the given commit, that its runs are over, and that
the file is exactly the list of the dashmate packages the release serves, downloaded into a new
empty directory.

Without --upload the signature is made and verified, then discarded, and the command that
attaches it is printed. With --upload every check runs again, the release is checked once
more right before the upload, and SHA256SUMS.asc is attached and read back.

  Options:
  --tag=TAG                  - release tag, e.g. v5.1.0
  --key=FINGERPRINT          - full fingerprint of the primary key to sign with; spaces are
                               allowed. gpg signs with its newest signing subkey.
  --commit=SHA               - full sha of the release PR's merge commit; the tag and the newest
                               release.yml run of the tag must both name it
  --upload                   - attach SHA256SUMS.asc to the release
  --repo=OWNER/NAME          - repository, default dashpay/platform
  -h          --help         - show help
"

# The keys whose signatures the apt repository accepts, by primary key fingerprint. This list is
# the one the release skill points to.
ALLOWED_SIGNERS=(
  19E460D55429A91B40E9E7C5EDBC0AA6BC1AA13C
)
# QuantumExplorer's key is added above once it exists.

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
KEY=""
COMMIT=""
UPLOAD=false
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
    --key=*)
      KEY="${i#*=}"
      ;;
    --commit=*)
      COMMIT="${i#*=}"
      ;;
    --upload)
      UPLOAD=true
      ;;
    --repo=*)
      REPO="${i#*=}"
      ;;
    *)
      usage_error "unknown option ${i}"
      ;;
  esac
done

# Matched as one string, so a value spanning lines cannot pass on one of them.
tag_pattern='^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$'
if ! [[ "${TAG}" =~ ${tag_pattern} ]]; then
  usage_error "--tag must be a release tag such as v5.1.0, got '${TAG}'"
fi

# A short key id can match more than one key, so only a full fingerprint is accepted.
KEY="$(echo "${KEY}" | tr -d ' ' | tr 'a-f' 'A-F')"
key_pattern='^([0-9A-F]{40}|[0-9A-F]{64})$'
if ! [[ "${KEY}" =~ ${key_pattern} ]]; then
  usage_error "--key must be the key's full fingerprint, got '${KEY}'"
fi

COMMIT="$(echo "${COMMIT}" | tr 'A-F' 'a-f')"
commit_pattern='^[0-9a-f]{40}$'
if ! [[ "${COMMIT}" =~ ${commit_pattern} ]]; then
  usage_error "--commit must be the full sha of the release PR's merge commit, got '${COMMIT}'"
fi

# Checked first: with no accepted signer nothing can be signed, and an empty array also cannot be
# expanded under set -u in bash 3.2.
[ "${#ALLOWED_SIGNERS[@]}" -gt 0 ] || fail "ALLOWED_SIGNERS lists no key, so this script accepts no signature"

for tool in gh gpg; do
  command -v "${tool}" > /dev/null 2>&1 || fail "${tool} is not installed"
done

# GNU sha256sum, or shasum where it is missing. The sha256sum macOS ships is another tool,
# which passes checks GNU's refuses.
if sha256sum --version 2> /dev/null | grep -q 'GNU coreutils'; then
  sha256=(sha256sum)
elif command -v shasum > /dev/null 2>&1; then
  sha256=(shasum -a 256)
else
  fail "neither GNU sha256sum nor shasum is installed"
fi

# pinentry asks for the passphrase in this terminal when there is one.
if [ -t 0 ]; then
  GPG_TTY="$(tty)"
  export GPG_TTY
fi

hash_of() {
  "${sha256[@]}" "$1" | cut -d' ' -f1
}

# One line per asset: name, state, uploader, digest, id.
release_assets() {
  gh api --paginate "repos/${REPO}/releases/${RELEASE_ID}/assets?per_page=100" \
    --jq '.[] | [.name, .state, .uploader.login, (.digest // ""), .id] | @tsv'
}

# A signature already on the release means another maintainer signed it, or the release was
# rebuilt without removing the old one. Either way a person has to look.
refuse_if_signed() {
  local signatures
  signatures="$(echo "$1" | awk -F'\t' '$1 ~ /^SHA256SUMS\./ {print $1}' | paste -sd ' ' -)"
  if [ -n "${signatures}" ]; then
    fail "${TAG} already carries ${signatures}. To sign it again, delete that asset first: gh release delete-asset ${TAG} <name> --repo ${REPO}"
  fi
}

# The fields of SHA256SUMS in a release_assets listing: state, uploader, digest, id.
sums_asset() {
  echo "$1" | awk -F'\t' '$1 == "SHA256SUMS" {print $2 "\t" $3 "\t" $4 "\t" $5}'
}

# Every release.yml run of the tag must be over: a running attempt can still replace packages
# or SHA256SUMS. Prints the newest run's id and the commit it built. Reports a refusal through
# the function named by $1, `fail` by default.
finished_run() {
  local on_fail="${1:-fail}" runs unfinished
  runs="$(gh api --paginate "repos/${REPO}/actions/workflows/release.yml/runs?event=release&branch=${TAG}&per_page=100" \
    --jq '.workflow_runs[] | [.id, .status, .head_sha] | @tsv')" || "${on_fail}" "could not list the release.yml runs of ${TAG}"
  [ -n "${runs}" ] || "${on_fail}" "found no release.yml run for ${TAG}"
  unfinished="$(echo "${runs}" | awk -F'\t' '$2 != "completed" {print $1 " (" $2 ")"}' | paste -sd ' ' -)"
  [ -z "${unfinished}" ] || "${on_fail}" "release.yml for ${TAG} is still running: ${unfinished}; sign once no attempt is in progress"
  echo "${runs}" | head -n 1 | cut -f1,3
}

# The newest run (id, commit) and the tag must both name the commit the release was approved at.
# Reports a refusal through the function named by $2, `fail` by default.
check_commit() {
  local on_fail="${2:-fail}" tagged
  [ "$(echo "$1" | cut -f2)" = "${COMMIT}" ] \
    || "${on_fail}" "release.yml run $(echo "$1" | cut -f1) built $(echo "$1" | cut -f2), not ${COMMIT}"
  tagged="$(gh api "repos/${REPO}/commits/refs/tags/${TAG}" --jq '.sha')" || "${on_fail}" "could not resolve ${TAG} to a commit"
  [ "${tagged}" = "${COMMIT}" ] || "${on_fail}" "${TAG} names ${tagged}, not ${COMMIT}"
}

# A job's log as GitHub stores it. The runner colours the commands it echoes there, and newer gh
# versions refuse a response with terminal escape sequences unless told to allow them; older
# versions do not know that option.
job_log() {
  local help
  # Read whole: a reader that stopped at the first match would cut gh off, and pipefail would
  # then report the check as failed.
  help="$(gh api --help 2> /dev/null || true)"
  case "${help}" in
    *--allow-escape-sequences*) gh api --allow-escape-sequences "repos/${REPO}/actions/jobs/$1/logs" ;;
    *) gh api "repos/${REPO}/actions/jobs/$1/logs" ;;
  esac
}

# The id and conclusion of the Release Dashmate checksums job of a run, in its latest attempt.
checksums_job() {
  gh api --paginate "repos/${REPO}/actions/runs/$1/jobs?per_page=100" \
    --jq '.jobs[] | select(.name == "Release Dashmate checksums") | [.id, (.conclusion // "none")] | @tsv'
}

# The digests GitHub reports for the release's dashmate assets must be exactly SHA256SUMS, byte
# for byte. Prints the difference and returns 1 when they are not.
check_published_packages() {
  local published="${WORK_DIR}/published"
  echo "$1" | awk -F'\t' '$1 ~ /^dashmate/ {
      print ($2 == "uploaded" && $4 ~ /^sha256:/ ? substr($4, 8) : "missing") "  " $1
    }' | LC_ALL=C sort -k2,2 > "${published}"
  if ! cmp -s "${published}" "$2"; then
    diff "${published}" "$2" >&2 || true
    return 1
  fi
}

# After the upload the signature stays on the release whatever fails, so the message names it and
# how to remove it before trying again.
fail_attached() {
  fail "SHA256SUMS.asc is attached to ${TAG}, but $*. Delete it before trying again: gh release delete-asset ${TAG} SHA256SUMS.asc --repo ${REPO}"
}

verify_signature() {
  local dir="$1" status
  if ! status="$(gpg --status-fd 1 --verify "${dir}/SHA256SUMS.asc" "${dir}/SHA256SUMS")"; then
    fail "gpg did not verify ${dir}/SHA256SUMS.asc against ${dir}/SHA256SUMS"
  fi
  # More than one, for instance from a local-user line in gpg.conf, would put other keys' names
  # on the release.
  [ "$(echo "${status}" | grep -c '^\[GNUPG:\] NEWSIG')" = 1 ] \
    || fail "expected exactly one signature on SHA256SUMS; check gpg.conf for local-user or default-key lines"
  # GOODSIG only for a good signature by a key that is neither expired nor revoked.
  echo "${status}" | grep -q '^\[GNUPG:\] GOODSIG ' || fail "gpg reported no good signature on SHA256SUMS"
  # VALIDSIG names the signing key third and its primary key twelfth.
  if ! echo "${status}" | awk -v key="${KEY}" '$2 == "VALIDSIG" && ($3 == key || $12 == key) {found = 1} END {exit !found}'; then
    fail "the signature on SHA256SUMS was not made by ${KEY} but by $(echo "${status}" | awk '$2 == "VALIDSIG" {print $3 " (primary key " $12 ")"}' | paste -sd ' ' -); pass the primary key's fingerprint"
  fi
  local primary signer
  primary="$(echo "${status}" | awk '$2 == "VALIDSIG" {print $12}')"
  for signer in "${ALLOWED_SIGNERS[@]}"; do
    [ "${primary}" != "${signer}" ] || return 0
  done
  fail "${primary} is not one of the signers this script accepts (ALLOWED_SIGNERS); the apt repository takes signatures from those keys only"
}

RELEASE_ID="$(gh api "repos/${REPO}/releases/tags/${TAG}" --jq '.id')" || fail "no release ${TAG} in ${REPO}"
ASSETS="$(release_assets)" || fail "could not list the assets of ${TAG}"

refuse_if_signed "${ASSETS}"

SUMS="$(sums_asset "${ASSETS}")"
[ -n "${SUMS}" ] || fail "${TAG} has no SHA256SUMS; wait for the Release Dashmate checksums job of its release.yml run"
SUMS_STATE="$(echo "${SUMS}" | cut -f1)"
SUMS_UPLOADER="$(echo "${SUMS}" | cut -f2)"
SUMS_DIGEST="$(echo "${SUMS}" | cut -f3)"
SUMS_ID="$(echo "${SUMS}" | cut -f4)"
[ "${SUMS_STATE}" = uploaded ] || fail "SHA256SUMS on ${TAG} is not fully uploaded (state ${SUMS_STATE})"
[ "${SUMS_UPLOADER}" = 'github-actions[bot]' ] || fail "SHA256SUMS on ${TAG} was uploaded by ${SUMS_UPLOADER}, not by the release workflow"

RUN="$(finished_run)" || exit 1
RUN_ID="$(echo "${RUN}" | cut -f1)"
check_commit "${RUN}"
JOB="$(checksums_job "${RUN_ID}")" || fail "could not list the jobs of release.yml run ${RUN_ID}"
JOB_ID="$(echo "${JOB}" | cut -f1)"
[ "$(echo "${JOB}" | cut -f2)" = success ] \
  || fail "the Release Dashmate checksums job of run ${RUN_ID} did not succeed ($(echo "${JOB}" | cut -f2 | grep . || echo 'not found'))"

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/dashmate-sign.XXXXXX")"
trap 'rm -rf "${WORK_DIR}"' EXIT
DIR="${WORK_DIR}/release"
mkdir "${DIR}"

# The checksums job writes into its own log the id and sha256 of the SHA256SUMS it attached, and
# nothing else writes to that log. Only that asset is signed.
LOG="$(job_log "${JOB_ID}")" \
  || fail "could not read the log of the Release Dashmate checksums job (job ${JOB_ID} of run ${RUN_ID}). GitHub keeps job logs only as long as the repository's retention setting says, and re-runs a job only within 30 days of its run; past either, this release cannot be signed with this script"
RECORD="$(echo "${LOG}" | sed -nE 's/^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:.]+Z SHA256SUMS attached: asset ([0-9]+) sha256 ([0-9a-f]{64})$/\1 \2/p')"
if [ -z "${RECORD}" ] || [ "$(echo "${RECORD}" | wc -l | tr -d ' ')" != 1 ]; then
  fail "the log of the Release Dashmate checksums job (job ${JOB_ID} of run ${RUN_ID}) does not record exactly one SHA256SUMS"
fi
RECORD_ID="${RECORD%% *}"
RECORD_SUM="${RECORD#* }"
if [ "${SUMS_ID}" != "${RECORD_ID}" ] || [ "${SUMS_DIGEST}" != "sha256:${RECORD_SUM}" ]; then
  fail "SHA256SUMS on ${TAG} (asset ${SUMS_ID}, ${SUMS_DIGEST:-no digest}) is not the file the Release Dashmate checksums job ${JOB_ID} of run ${RUN_ID} attached (asset ${RECORD_ID}, sha256:${RECORD_SUM})"
fi

echo "Downloading the SHA256SUMS of ${TAG} into ${DIR}"
gh release download "${TAG}" --repo "${REPO}" --pattern SHA256SUMS --dir "${DIR}" \
  || fail "could not download the SHA256SUMS of ${TAG}"

SIGNED_DIGEST="sha256:$(hash_of "${DIR}/SHA256SUMS")"
[ "${SIGNED_DIGEST}" = "${SUMS_DIGEST}" ] || fail "the downloaded SHA256SUMS is not the file the release lists (${SIGNED_DIGEST}, release ${SUMS_DIGEST:-none})"

# The file must be exactly what the release workflow writes for the dashmate assets the release
# serves, by the digests GitHub reports: one sha256 line per package, sorted by name. Only then
# are those packages downloaded, each by its name.
check_published_packages "${ASSETS}" "${DIR}/SHA256SUMS" \
  || fail "the dashmate packages ${TAG} serves are not the ones its SHA256SUMS lists (< served, > listed)"
patterns=()
while IFS= read -r name; do
  patterns+=(--pattern "${name}")
done < <(cut -d' ' -f3 "${DIR}/SHA256SUMS")
# Also keeps the expansion below from an empty array, which bash 3.2 rejects under set -u.
[ "${#patterns[@]}" -gt 0 ] || fail "SHA256SUMS of ${TAG} lists no packages"
echo "Downloading the $(( ${#patterns[@]} / 2 )) dashmate packages of ${TAG} into ${DIR}"
gh release download "${TAG}" --repo "${REPO}" "${patterns[@]}" --dir "${DIR}" \
  || fail "could not download the packages of ${TAG}"

# Every line, strictly, against the bytes downloaded: a missing package or other bytes fail.
( cd "${DIR}" && "${sha256[@]}" --check --strict SHA256SUMS ) || fail "the packages of ${TAG} do not match its SHA256SUMS"

echo "Signing SHA256SUMS with ${KEY}"
gpg --armor --detach-sign --local-user "${KEY}" --output "${DIR}/SHA256SUMS.asc" "${DIR}/SHA256SUMS" \
  || fail "gpg could not sign SHA256SUMS with ${KEY}"
verify_signature "${DIR}"

echo "SHA256SUMS of ${TAG} matches its packages and is signed by ${KEY}."

if [ "${UPLOAD}" != true ]; then
  repo_option=""
  [ "${REPO}" = dashpay/platform ] || repo_option=" --repo=${REPO}"
  echo "Not uploaded. To attach the signature, run:"
  echo "  scripts/release/sign_dashmate_checksums.sh --tag=${TAG} --key=${KEY} --commit=${COMMIT}${repo_option} --upload"
  exit 0
fi

# Checked again right before the upload, since the passphrase prompt can take a while: the same
# run and checksums job, nobody signed, and SHA256SUMS and the packages are still the signed ones.
LATEST_RUN="$(finished_run)" || exit 1
check_commit "${LATEST_RUN}"
LATEST_JOB="$(checksums_job "${RUN_ID}")" || fail "could not list the jobs of release.yml run ${RUN_ID}"
if [ "${LATEST_RUN}" != "${RUN}" ] || [ "${LATEST_JOB}" != "${JOB}" ]; then
  fail "the release.yml run or its Release Dashmate checksums job changed while SHA256SUMS was being signed; run this again"
fi
ASSETS="$(release_assets)" || fail "could not list the assets of ${TAG}"
refuse_if_signed "${ASSETS}"
SUMS="$(sums_asset "${ASSETS}")"
if [ "$(echo "${SUMS}" | cut -f3)" != "${SIGNED_DIGEST}" ] || [ "$(echo "${SUMS}" | cut -f4)" != "${RECORD_ID}" ]; then
  fail "SHA256SUMS on ${TAG} changed while it was being signed; run this again"
fi
check_published_packages "${ASSETS}" "${DIR}/SHA256SUMS" \
  || fail "the dashmate packages ${TAG} serves are not the ones its SHA256SUMS lists (< served, > listed)"

READ_BACK="${WORK_DIR}/read-back"
mkdir "${READ_BACK}"

# The upload fails when the release already carries a SHA256SUMS.asc, and a failed request can
# still have created the asset. It goes on only when the release serves the signature made here;
# anything else attached is left for the maintainer to inspect.
if ! gh release upload "${TAG}" "${DIR}/SHA256SUMS.asc" --repo "${REPO}"; then
  ASSETS="$(release_assets)" \
    || fail "the upload of SHA256SUMS.asc to ${TAG} failed, and its assets could not be listed. Check what is attached with gh release view ${TAG} --repo ${REPO} before running this again"
  ATTACHED="$(echo "${ASSETS}" | awk -F'\t' '$1 ~ /^SHA256SUMS\./ {print $1 " (" $2 ")"}' | paste -sd ' ' -)"
  [ -n "${ATTACHED}" ] || fail "the upload of SHA256SUMS.asc to ${TAG} failed and nothing is attached; run this again"
  INSPECT="Check what is attached with gh release view ${TAG} --repo ${REPO} and gpg --verify before changing anything"
  [ "${ATTACHED}" = 'SHA256SUMS.asc (uploaded)' ] \
    || fail "the upload of SHA256SUMS.asc to ${TAG} failed, and ${TAG} now carries ${ATTACHED} instead of a complete SHA256SUMS.asc. ${INSPECT}"
  mkdir "${WORK_DIR}/attached"
  gh release download "${TAG}" --repo "${REPO}" --pattern SHA256SUMS.asc --dir "${WORK_DIR}/attached" \
    || fail "the upload of SHA256SUMS.asc to ${TAG} failed, and the SHA256SUMS.asc now attached could not be downloaded to compare it with the one signed here. ${INSPECT}"
  cmp -s "${DIR}/SHA256SUMS.asc" "${WORK_DIR}/attached/SHA256SUMS.asc" \
    || fail "the upload of SHA256SUMS.asc to ${TAG} failed, and ${TAG} carries a SHA256SUMS.asc that is not the one signed here. ${INSPECT}"
  echo "The upload reported a failure, but ${TAG} serves the SHA256SUMS.asc signed here."
fi

# Read back: the release serves the files that were signed and verified.
gh release download "${TAG}" --repo "${REPO}" --pattern SHA256SUMS --pattern SHA256SUMS.asc --dir "${READ_BACK}" \
  || fail_attached "could not download SHA256SUMS and SHA256SUMS.asc back from it"
for file in SHA256SUMS SHA256SUMS.asc; do
  cmp -s "${DIR}/${file}" "${READ_BACK}/${file}" || fail_attached "${TAG} serves another ${file} than the one signed here"
done
ASSETS="$(release_assets)" || fail_attached "could not list its assets"
check_published_packages "${ASSETS}" "${DIR}/SHA256SUMS" \
  || fail_attached "the dashmate packages it serves are not the ones its SHA256SUMS lists (< served, > listed)"
# The tag, the newest run or its checksums job can have changed while the signature was uploaded:
# success only for the same run and job, still approved at --commit.
FINAL_RUN="$(finished_run fail_attached)" || exit 1
check_commit "${FINAL_RUN}" fail_attached
FINAL_JOB="$(checksums_job "${RUN_ID}")" || fail_attached "the jobs of release.yml run ${RUN_ID} could not be listed"
if [ "${FINAL_RUN}" != "${RUN}" ] || [ "${FINAL_JOB}" != "${JOB}" ]; then
  fail_attached "the release.yml run or its Release Dashmate checksums job changed while SHA256SUMS.asc was uploaded"
fi

echo "Attached SHA256SUMS.asc to ${TAG}; the release serves the signed SHA256SUMS and the packages it lists."
