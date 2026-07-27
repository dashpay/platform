/**
 * Prerelease identifiers Dash releases publish image tags for.
 *
 * Derived image tags are the major, optionally followed by one of these — `4`
 * on a stable release, `4-rc` on a release candidate. See
 * configs/defaults/getBaseConfigFactory.js for where the tag is derived.
 */
export const STOCK_PRERELEASE_IDS = ['alpha', 'beta', 'dev', 'hotfix', 'pr', 'rc'];

const stockPrereleaseSuffix = `(-(${STOCK_PRERELEASE_IDS.join('|')}))?`;

/**
 * Match a tag a release of the given major published, and nothing else.
 *
 * The identifiers are listed rather than matched loosely because anything
 * broader also matches tags an operator set themselves in the same namespace:
 * a locally built `dashpay/drive:4-local` is indistinguishable in shape from a
 * published one. An identifier a later release invents is skipped rather than
 * guessed at, which leaves the operator's image alone instead of overwriting
 * it.
 *
 * @param {string} repository - image repository, e.g. 'dashpay/drive'
 * @param {number} major
 * @return {RegExp}
 */
export function stockImagePattern(repository, major) {
  const escapedRepository = repository.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  return new RegExp(`^${escapedRepository}:${major}${stockPrereleaseSuffix}$`);
}
