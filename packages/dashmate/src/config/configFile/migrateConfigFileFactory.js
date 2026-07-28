import semver from 'semver';

import getConfigFormatVersion from './getConfigFormatVersion.js';

export default function migrateConfigFileFactory(getConfigFileMigrations) {
  /**
   * @typedef {function} migrateConfigFile
   * @param {Object} rawConfigFile
   * @param {string} fromVersion
   * @param {string} toVersion
   * @returns {Object}
   */
  function migrateConfigFile(rawConfigFile, fromVersion, toVersion) {
    const configFileMigrations = getConfigFileMigrations();

    // Migrate towards the format this build produces, which during a
    // development cycle is ahead of the released version. Recording the result
    // is what keeps it deterministic: stamping the older version would leave a
    // config claiming a format whose migrations it has already run, and the next
    // upgrade would skip them.
    const targetVersion = getConfigFormatVersion(configFileMigrations, toVersion);

    if (semver.gte(fromVersion, targetVersion)) {
      return rawConfigFile;
    }

    /**
     * @type {Object}
     */
    const migratedConfigFile = Object.keys(configFileMigrations)
      .filter((version) => semver.gt(version, fromVersion) && semver.lte(version, targetVersion))
      .sort(semver.compare)
      .reduce((migratedOptions, version) => {
        const migrationFunction = configFileMigrations[version];

        // Thread the accumulator: migrations mutate in place and return the same
        // object today, but a migration returning a new one would otherwise have
        // its result dropped along with every earlier step.
        return migrationFunction(migratedOptions);
      }, rawConfigFile);

    migratedConfigFile.configFormatVersion = targetVersion;

    return migratedConfigFile;
  }

  return migrateConfigFile;
}
