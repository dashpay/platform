import semver from 'semver';

export default function migrateConfigFileFactory(getConfigFileMigrations) {
  /**
   * @typedef {function} migrateConfigFile
   * @param {Object} rawConfigFile
   * @param {string} fromVersion
   * @param {string} toVersion
   * @returns {Object}
   */
  function migrateConfigFile(rawConfigFile, fromVersion, toVersion) {
    if (fromVersion === toVersion) {
      return rawConfigFile;
    }

    const configFileMigrations = getConfigFileMigrations();

    /**
     * @type {Object}
     */
    const migratedConfigFile = Object.keys(configFileMigrations)
      .filter((version) => semver.gt(version, fromVersion))
      .sort(semver.compare)
      .reduce((migratedOptions, version) => {
        const migrationFunction = configFileMigrations[version];
        return migrationFunction(rawConfigFile);
      }, rawConfigFile);

    migratedConfigFile.configFormatVersion = toVersion;

    return migratedConfigFile;
  }

  return migrateConfigFile;
}
