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
 * Match a tag published by a release of any version, for migrations that move
 * images across majors.
 *
 * Historical tags took several shapes - `0.25.16`, `1-dev`, `3`, `4-rc` - so the
 * numeric part is permissive, while the prerelease identifier stays restricted
 * to the published list. Without that restriction an operator's own tag such as
 * `dashpay/drive:4-local` would match and be overwritten.
 *
 * @param {string} repository - image repository, e.g. 'dashpay/drive'
 * @return {RegExp}
 */
export function stockImagePatternAnyVersion(repository) {
  const escapedRepository = repository.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // Published tags only ever took two shapes: the 0.x line used a major.minor
  // tag (`0.25`), every line since uses the major alone. Anything more
  // permissive matches an operator's own exact pin such as `dashpay/drive:3.1.5`,
  // which was never a published default.
  return new RegExp(`^${escapedRepository}:(0\\.\\d+|\\d+)${stockPrereleaseSuffix}$`);
}
