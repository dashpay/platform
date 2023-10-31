const semver = require('semver');

function migrateConfigFileFactory(getConfigFileMigrations) {
  /**
   * @typedef {function} migrateConfigFile
   * @param {Object} rawConfigFile
   * @param {string} fromVersion
   * @param {string} toVersion
   * @returns {Object}
   */
  function migrateConfigFile(rawConfigFile, fromVersion, toVersion) {
    // TODO: We just need to migrate up to the latest version in migrations
    //  to handle properly development process when you work on non-released version
    if (fromVersion === toVersion) {
      return rawConfigFile;
    }

    const configFileMigrations = getConfigFileMigrations();

    return Object.keys(configFileMigrations)
      .filter((version) => semver.gt(version, fromVersion))
      .sort(semver.compare)
      .reduce((migratedOptions, version) => {
        const migrationFunction = configFileMigrations[version];
        return migrationFunction(rawConfigFile);
      }, rawConfigFile);
  }

  return migrateConfigFile;
}

module.exports = migrateConfigFileFactory;
