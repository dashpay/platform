import AbstractError from '../../errors/AbstractError.js';

/**
 * The config file records a newer format than this build produces, so a newer
 * dashmate has already read and saved it.
 *
 * Migration only runs forward: this build reads such a file without migrating
 * it, and saving would write its own state - and render its own service files -
 * under the newer stamp, where the newer build never migrates it again.
 */
export default class ConfigFileFormatTooNewError extends AbstractError {
  /**
   * @param {string} configFilePath
   * @param {string} recordedVersion - the format recorded in the file
   * @param {string} ownVersion - the format this build produces
   * @param {Object} [options={}]
   * @param {boolean} [options.isBeforeAnyWork=false] - refused when the lock was
   *   taken, before the caller did anything, rather than at a later save
   */
  constructor(configFilePath, recordedVersion, ownVersion, { isBeforeAnyWork = false } = {}) {
    super(`Config file '${configFilePath}' was updated by a newer dashmate (config format`
      + ` ${recordedVersion}; this dashmate writes config format ${ownVersion}). Nothing was saved,`
      + ' so the newer settings were not overwritten with older ones. Run this command with the'
      + ' newer dashmate; `which -a dashmate` lists every copy on your PATH. The dashmate helper'
      + ' runs the version that started the node, so it refuses the same way until the node runs'
      + ' the newer version. Downgrading dashmate is not supported.');

    this.configFilePath = configFilePath;
    this.recordedVersion = recordedVersion;
    this.ownVersion = ownVersion;
    this.isBeforeAnyWork = isBeforeAnyWork;
  }
}
