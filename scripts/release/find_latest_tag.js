const semver = require('semver');
const execute = require('../utils/execute');

/**
 * Choose the tag the changelog should be generated from for a given target version.
 *
 * The base must be the immediately-preceding release so conventional-changelog only
 * emits the delta for the new version. Picking a base that is too far back makes it
 * regenerate the intervening version sections and prepend them on top of the copies
 * already in CHANGELOG.md, duplicating them.
 *
 * `tags` must be ordered newest-created-first (git tag --sort=-creatordate). Release
 * chronology is used rather than semver precedence on purpose: prerelease ids do not
 * sort by release order (semver compares them alphabetically, so e.g. "beta" < "dev"),
 * so the last prerelease actually released before a new one is only knowable from
 * creation order.
 *
 * Resolution order:
 *   prerelease vX.Y.0-<id>.N
 *     1. newest prerelease on the same X.Y.0 line sharing the same <id>
 *     2. newest prerelease of any id on the same X.Y.0 line
 *        (so the first rc after betas bases off the last beta, not the previous stable)
 *     3. latest stable release of the previous minor
 *   stable vX.Y.Z
 *     1. latest stable release of the same minor
 *     2. newest prerelease on the same X.Y.0 line (a stable cut from a prerelease line)
 *     3. latest stable release of the previous minor
 *
 * @param {string} version - target version, with or without a leading "v"
 * @param {string[]} tags - existing git tags, ordered newest-created-first
 * @returns {string|null} the chosen tag, or null if none matches
 */
function findLatestTag(version, tags) {
  const parsed = semver.parse(version);

  if (!parsed) {
    throw new Error(`Invalid version: ${version}`);
  }

  const { major, minor } = parsed;
  const isPrerelease = parsed.prerelease.length > 0;

  // Preserve the given (newest-first) order; drop unparseable tags and the target itself.
  const candidates = tags
    .map((tag) => ({ tag, semver: semver.parse(tag) }))
    .filter(({ semver: v }) => v && v.version !== parsed.version);

  const first = (predicate) => {
    const match = candidates.find(({ semver: v }) => predicate(v));
    return match ? match.tag : null;
  };

  const sameLinePrerelease = (v) => v.prerelease.length > 0 && v.major === major && v.minor === minor;
  const stableOfMinor = (targetMinor) => (v) => v.prerelease.length === 0
    && v.major === major
    && v.minor === targetMinor;

  if (isPrerelease) {
    const [preId] = parsed.prerelease;

    return first((v) => sameLinePrerelease(v) && v.prerelease[0] === preId)
      || first(sameLinePrerelease)
      || first(stableOfMinor(minor - 1));
  }

  return first(stableOfMinor(minor))
    || first(sameLinePrerelease)
    || first(stableOfMinor(minor - 1));
}

/**
 * Existing tags created after the chosen base (i.e. newer than it) other than the
 * target. Each already has its own CHANGELOG.md section, so regenerating from `base`
 * would duplicate them — a signal that `base` is wrong.
 *
 * @param {string} version - target version
 * @param {string} base - chosen base tag
 * @param {string[]} tags - existing git tags, ordered newest-created-first
 * @returns {string[]} intervening tags, newest first
 */
function interveningTags(version, base, tags) {
  const parsedTarget = semver.parse(version);
  const baseIndex = tags.indexOf(base);

  if (baseIndex === -1) {
    return [];
  }

  return tags
    .slice(0, baseIndex)
    .filter((tag) => {
      const v = semver.parse(tag);
      return v && (!parsedTarget || v.version !== parsedTarget.version);
    });
}

module.exports = { findLatestTag, interveningTags };

if (require.main === module) {
  const [version] = process.argv.slice(2);

  if (!version) {
    console.error('usage example: yarn node find_latest_tag.js v0.21.0');
    process.exit(1);
  }

  (async () => {
    const rawTags = await execute('git tag -l --sort=-creatordate');
    const tags = rawTags.split('\n').map((tag) => tag.trim()).filter(Boolean);

    const result = findLatestTag(version, tags);

    if (!result) {
      console.error(`Can't find latest tag for the version ${version}`);
      process.exit(1);
    }

    const intervening = interveningTags(version, result, tags);
    if (intervening.length > 0) {
      console.error(`WARNING: existing tags newer than the changelog base (${result}) already have `
        + `CHANGELOG.md sections and will be duplicated: ${intervening.join(', ')}. `
        + `Pass an explicit -c=<tag> if this base is wrong.`);
    }

    console.log(result);
  })();
}
