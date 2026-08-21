import AbstractError from '../../errors/AbstractError.js';

/**
 * The configuration file needs migrating, and this caller promised not to
 * change anything.
 *
 * Migrations are not all pure: one copies TLS material to a new location,
 * removes the originals and then deletes the whole legacy ssl directory. Doing
 * that on behalf of a command documented as safe to run against a node that is
 * still up - without holding the configuration lock, and without recording the
 * result, so it would happen again on the next run - is worse than declining.
 */
export default class ConfigFileMigrationRequiredError extends AbstractError {
  /**
   * @param {string} configFilePath
   */
  constructor(configFilePath) {
    // Wrapped short: this reaches the operator through oclif's error printer,
    // which hard-wraps at the terminal width less six and breaks mid-token.
    super(`This node's configuration was written by an older dashmate
and has to be migrated before it can be read:

    ${configFilePath}

Migrating moves and removes files on disk, so a command
that changes nothing will not do it.

Run any other dashmate command first, for example:

    dashmate status

That migrates the configuration while holding the
configuration lock. Then run this one again.`);

    this.configFilePath = configFilePath;
  }

  /**
   * @return {string}
   */
  getConfigFilePath() {
    return this.configFilePath;
  }
}
