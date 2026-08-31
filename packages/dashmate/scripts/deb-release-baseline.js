/**
 * Choose the published release whose Debian package a new release has to sort above.
 *
 * apt refuses a package whose version does not sort above the one already installed, so
 * the release to measure against is the last one an operator was offered on the same
 * release line - or, when the line has not shipped a package yet, the last one offered
 * at all. The release being built is deliberately a candidate: a package already
 * attached to that tag is what an earlier run shipped, and it is exactly what apt
 * compares a rebuild against.
 *
 * Only the choice of release is made here. The version itself is always read from the
 * chosen package's own control field, never derived from the tag, because that is the
 * only version apt looks at and the only one a rename of the asset cannot alter.
 */

import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

// Releases are ordered by when they were published rather than when they were created.
// A release drafted early and published late reaches operators after releases created
// after it, and it is the order the packages were offered in that decides what apt has
// already installed. Ordering by creation lets a rerun of such a release measure itself
// against its own predecessor instead of against the package it already shipped, which
// passes a same-version rebuild that apt will then report as already the newest version.
const PUBLISHED_AT = 'published_at';

// Only the architecture-independent naming matters here: any published package can be
// read for its version, and the amd64 one is present in every release that shipped debs.
const BASELINE_ASSET_SUFFIX = '_amd64.deb';

/**
 * The release line a tag belongs to, as `major.minor`.
 *
 * Used only to group releases, so that a hotfix on an older line is measured against its
 * own predecessor instead of a higher line it was never meant to supersede.
 *
 * @param {string} tag
 * @returns {string}
 */
function releaseLine(tag) {
  return String(tag).replace(/^v/, '').split('-')[0].split('.').slice(0, 2).join('.');
}

/**
 * @param {object[]} releases - releases as returned by the GitHub releases API
 * @param {string} currentTag - the tag being released
 * @returns {{tag: string, asset: string}|null} the release to compare against, if any
 */
function selectDebBaseline(releases, currentTag) {
  const candidates = releases
    // Drafts have never been offered to anyone and carry no publication date to sort by.
    // Prereleases are kept: they go to the same channel, and apt compares against
    // whatever the operator installed last regardless of how it was labelled.
    .filter((release) => release.draft === false && release[PUBLISHED_AT] != null)
    .sort((left, right) => String(right[PUBLISHED_AT]).localeCompare(String(left[PUBLISHED_AT])))
    .map((release) => ({
      tag: release.tag_name,
      asset: (release.assets || [])
        .map((asset) => asset.name)
        .find((name) => String(name).endsWith(BASELINE_ASSET_SUFFIX)),
    }))
    .filter((candidate) => candidate.asset !== undefined);

  const line = releaseLine(currentTag);

  return candidates.find((candidate) => releaseLine(candidate.tag) === line)
    || candidates[0]
    || null;
}

export { selectDebBaseline, releaseLine };

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const currentTag = process.argv[2];

  if (!currentTag) {
    console.error('Usage: deb-release-baseline.js CURRENT_TAG < releases.json\n\n'
      + '  Reads the GitHub releases API response on stdin and prints the tag and the\n'
      + '  package file name of the release the current tag has to sort above, separated\n'
      + '  by a tab. Prints nothing when no published release has shipped a package.\n');

    process.exit(1);
  }

  const input = fs.readFileSync(0, 'utf8');
  const parsed = JSON.parse(input);

  if (!Array.isArray(parsed)) {
    console.error('Expected the releases API response to be an array');

    process.exit(1);
  }

  // A paginated response arrives as one array per page.
  const releases = parsed.flatMap((page) => (Array.isArray(page) ? page : [page]));

  const baseline = selectDebBaseline(releases, currentTag);

  if (baseline !== null) {
    process.stdout.write(`${baseline.tag}\t${baseline.asset}\n`);
  }
}
