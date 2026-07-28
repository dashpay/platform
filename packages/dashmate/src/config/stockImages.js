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

/**
 * Match a tag published by any release up to major 4, for migrations that move
 * images across majors.
 *
 * Frozen to the shapes that actually existed. The 0.x line and v1.0.0/v1.0.1
 * published major.minor tags (`0.25`, `1.0`, `1.0-rc`); the derivation changed
 * to the major alone in v1.0.2. The major is bounded because these migrations
 * are historical: a config carrying a tag from a later major never reaches
 * them, so accepting one could only ever match an operator's own image.
 *
 * @param {string} repository - image repository, e.g. 'dashpay/drive'
 * @return {RegExp}
 */
export function historicalStockImagePattern(repository) {
  const escapedRepository = repository.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  return new RegExp(`^${escapedRepository}:(0\\.\\d+|1\\.0|[1-4])${stockPrereleaseSuffix}$`);
}
