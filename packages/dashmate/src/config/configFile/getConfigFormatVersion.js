import semver from 'semver';

/**
 * The config format version this dashmate build produces.
 *
 * Normally the released version, but a migration is keyed at the release it will
 * ship in, so for the whole development cycle the build carries migrations - and
 * defaults matching them - above the version in package.json. A config written
 * or migrated by such a build is already in the newer shape, and recording the
 * older version would send it through those migrations again on the next command.
 *
 * @param {Object} configFileMigrations
 * @param {string} packageVersion
 * @return {string}
 */
export default function getConfigFormatVersion(configFileMigrations, packageVersion) {
  const newestMigration = Object.keys(configFileMigrations).sort(semver.compare).at(-1);

  return semver.gt(newestMigration, packageVersion) ? newestMigration : packageVersion;
}
