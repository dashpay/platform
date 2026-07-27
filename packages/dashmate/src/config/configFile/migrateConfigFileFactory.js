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
      // Deliberately unbounded above: migrations are keyed at the release they
      // will ship in, which is ahead of the version in package.json for the
      // whole development cycle. The table comes from the installed binary, so
      // a migration present in it belongs to that build whatever the package
      // version says - bounding by toVersion would leave a config half migrated
      // against defaults that already moved.
      .filter((version) => semver.gt(version, fromVersion))
      .sort(semver.compare)
      .reduce((migratedOptions, version) => {
        const migrationFunction = configFileMigrations[version];

        // Thread the accumulator: migrations mutate in place and return the same
        // object today, but a migration returning a new one would otherwise have
        // its result dropped along with every earlier step.
        return migrationFunction(migratedOptions);
      }, rawConfigFile);

    migratedConfigFile.configFormatVersion = toVersion;

    return migratedConfigFile;
  }

  return migrateConfigFile;
}
