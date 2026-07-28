import AbstractError from '../../errors/AbstractError.js';

/**
 * Raised when the config file changed on disk after this process loaded it.
 *
 * Writing would silently revert whatever the other process saved, so the write
 * is refused. The in-memory state is parked next to the config file first, so
 * material generated during the command - operator keys, a spork key, a miner
 * address - is never lost with the refused write.
 */
export default class ConfigFileConflictError extends AbstractError {
  /**
   * @param {string} configFilePath
   * @param {string|null} rejectedSnapshotPath - null when parking it also failed
   * @param {Error} [snapshotError] - why parking failed, when it did
   */
  constructor(configFilePath, rejectedSnapshotPath, snapshotError = undefined) {
    const recovery = rejectedSnapshotPath === null
      ? `The changes from this command could NOT be saved either: ${snapshotError?.message}. They are lost.`
      : `The changes from this command were saved to '${rejectedSnapshotPath}' so you can reconcile them.`;

    super(`'${configFilePath}' was modified by another process after this command loaded it.`
      + ` Refusing to overwrite it. ${recovery}`);

    this.configFilePath = configFilePath;
    this.rejectedSnapshotPath = rejectedSnapshotPath;
    this.snapshotError = snapshotError;

    // Stable identifier for callers that need to branch on this without
    // depending on the class identity across module instances.
    this.code = 'DASHMATE_CONFIG_FILE_CONFLICT';
  }

  /**
   * @returns {string}
   */
  getConfigFilePath() {
    return this.configFilePath;
  }

  /**
   * Path the refused state was parked at, or null if that failed too.
   *
   * @returns {string|null}
   */
  getRejectedSnapshotPath() {
    return this.rejectedSnapshotPath;
  }
}
