import fs from 'fs';
import Ajv from 'ajv';
import path from 'path';
import lockfile from 'proper-lockfile';
import writeFileAtomic from 'write-file-atomic';
import Config from '../Config.js';
import { PACKAGE_ROOT_DIR } from '../../constants.js';
import ConfigFileNotFoundError from '../errors/ConfigFileNotFoundError.js';
import InvalidConfigFileFormatError from '../errors/InvalidConfigFileFormatError.js';
import configFileJsonSchema from './configFileJsonSchema.js';
import ConfigFile from './ConfigFile.js';

/**
 * How long a lock may go un-refreshed before another process may break it.
 *
 * A command that reconfigures a node holds this across its whole run, and the
 * refresh that keeps it alive cannot happen while the event loop is busy - so
 * this has to comfortably outlast the longest synchronous stretch such a command
 * has, or its live lock gets stolen and two processes write. The cost of the
 * generous value is that a process killed while holding the lock blocks the next
 * writer for this long.
 */
const LOCK_STALE_MS = 60000;

/**
 * How long to wait for someone else's lock before giving up.
 *
 * Deliberately much shorter than the stale threshold: waiting is synchronous, so
 * signal handlers do not run meanwhile, and an operator is better served by a
 * quick "something else is changing configuration" than by a long silent stall.
 */
const LOCK_ACQUIRE_TIMEOUT_MS = 15000;

const LOCK_RETRY_INTERVAL_MS = 50;

/**
 * Block without spinning. The write path is synchronous, so there is no event
 * loop to yield to here.
 *
 * @param {number} ms
 */
function sleepSync(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

export default class ConfigFileJsonRepository {
  /**
   * Release function for a lock held across a whole command, or null when this
   * process is not holding one.
   *
   * @type {function|null}
   */
  #heldRelease = null;

  /**
   * Set when the lock was lost while we believed we still held it. Continuing
   * would mean writing without exclusivity, which is the lost update this exists
   * to prevent, so the next save refuses instead.
   *
   * @type {boolean}
   */
  #compromised = false;

  /**
   * @param {migrateConfigFile} migrateConfigFile
   * @param {HomeDir} homeDir
   * @param {Object} [configFileLockOptions={}] - overrides for the lock timings,
   *   so the paths that only happen after seconds of waiting can be exercised
   * @param {number} [configFileLockOptions.stale]
   * @param {number} [configFileLockOptions.acquireTimeout]
   */
  constructor(migrateConfigFile, homeDir, configFileLockOptions = {}) {
    this.migrateConfigFile = migrateConfigFile;
    this.ajv = new Ajv();
    this.lockStaleMs = configFileLockOptions.stale ?? LOCK_STALE_MS;
    this.lockAcquireTimeoutMs = configFileLockOptions.acquireTimeout ?? LOCK_ACQUIRE_TIMEOUT_MS;
    this.configFilePath = homeDir.joinPath('config.json');
    // Locking a sibling rather than the config file itself keeps first run
    // working, where there is no config file to lock yet.
    this.lockFilePath = homeDir.joinPath('config.json.lock');
  }

  /**
   * Load configs from file
   *
   * @param {Object} [options={}]
   * @param {boolean} [options.skipValidation=false] - Skip per-config schema validation
   * @returns {ConfigFile}
   */
  read(options = {}) {
    const { skipValidation = false } = options;
    if (!fs.existsSync(this.configFilePath)) {
      throw new ConfigFileNotFoundError(this.configFilePath);
    }

    const configFileJSON = fs.readFileSync(this.configFilePath, 'utf8');

    let configFileData;
    try {
      configFileData = JSON.parse(configFileJSON);
    } catch (e) {
      throw new InvalidConfigFileFormatError(this.configFilePath, e);
    }

    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const originConfigVersion = configFileData.configFormatVersion;

    const migratedConfigFileData = this.migrateConfigFile(
      configFileData,
      configFileData.configFormatVersion,
      version,
    );

    const isValid = this.ajv.validate(configFileJsonSchema, migratedConfigFileData);

    if (!isValid) {
      const error = new Error(this.ajv.errorsText(undefined, { dataVar: 'configFile' }));

      throw new InvalidConfigFileFormatError(this.configFilePath, error);
    }

    let configs;
    try {
      configs = Object.entries(migratedConfigFileData.configs)
        .map(([name, opts]) => new Config(name, opts, skipValidation));
    } catch (e) {
      throw new InvalidConfigFileFormatError(this.configFilePath, e);
    }

    const configFile = new ConfigFile(
      configs,
      migratedConfigFileData.configFormatVersion,
      migratedConfigFileData.projectId,
      migratedConfigFileData.defaultConfigName,
      migratedConfigFileData.defaultGroupName,
    );

    // Mark configs as changed if they were migrated
    if (migratedConfigFileData.configFormatVersion !== originConfigVersion) {
      configFile.markAsChanged();
      configFile.getAllConfigs().forEach((config) => config.markAsChanged());
    }

    return configFile;
  }

  /**
   * Read, change and save the config file as one indivisible step.
   *
   * Everything happens while the file is locked, and the state handed to the
   * mutator is read after taking the lock - so a change made by another process
   * in between cannot be reverted, and two commands editing different options
   * both survive. Reading in one place and saving in another is what allowed a
   * command to write a snapshot that was already out of date.
   *
   * The lock is held for a read, a mutation and a rename - milliseconds - so it
   * is not a meaningful serialization point for commands that only edit config.
   *
   * @param {function(ConfigFile): void} mutate
   * @param {Object} [options={}] - passed through to read()
   * @returns {ConfigFile} the state that was saved
   */
  update(mutate, options = {}) {
    return this.#locked(() => {
      const configFile = this.read(options);

      mutate(configFile);

      this.#save(configFile);

      return configFile;
    });
  }

  /**
   * Hold the lock for as long as this process is changing configuration.
   *
   * For a command that reconfigures a node over minutes, taking the lock before
   * it reads is what makes its state current: there is no window between reading
   * and saving for another process to write into. Short changes should use
   * update() instead - this blocks every other writer until release() is called.
   *
   * Calling it again while already held does nothing, so a command holding the
   * lock can still call update() and write() normally.
   *
   * @returns {void}
   */
  acquire() {
    if (this.#heldRelease !== null) {
      return;
    }

    this.#compromised = false;
    this.#heldRelease = this.#acquireLock();
  }

  /**
   * Give up a lock taken with acquire(). Safe to call when not holding one, so
   * it can be used from every exit path without checking first.
   *
   * @returns {void}
   */
  release() {
    if (this.#heldRelease === null) {
      return;
    }

    const release = this.#heldRelease;

    this.#heldRelease = null;

    this.#release(release);
  }

  /**
   * Run fn with the lock held, taking it only if this process is not already
   * holding one across a command.
   *
   * @param {function(): *} fn
   * @returns {*}
   */
  #locked(fn) {
    if (this.#heldRelease !== null) {
      return fn();
    }

    const release = this.#acquireLock();

    try {
      return fn();
    } finally {
      this.#release(release);
    }
  }

  /**
   * Save configs to file
   *
   * Prefer update() for changing configuration: it reads the current state under
   * the same lock, so it cannot save something another process has already
   * moved on from. This remains for callers that mutate a config file they are
   * holding and then save it as a separate step.
   *
   * @param {ConfigFile} configFile
   * @returns {void}
   */
  write(configFile) {
    this.#locked(() => this.#save(configFile));
  }

  /**
   * Serialize and replace the file, then mark the state clean.
   *
   * Caller must hold the lock.
   *
   * @param {ConfigFile} configFile
   */
  #save(configFile) {
    if (this.#compromised) {
      throw new Error(`Lost the lock on '${this.configFilePath}' while changing it,`
        + ' so saving now could overwrite another process. Nothing was written -'
        + ' re-run the command.');
    }

    const configFileJSON = `${JSON.stringify(configFile.toObject(), undefined, 2)}\n`;

    writeFileAtomic.sync(this.configFilePath, configFileJSON, 'utf8');

    // Only now is the state actually on disk. Clearing these beforehand would
    // leave the configs claiming to be saved after a failed write.
    configFile.markAsSaved();
    configFile.getAllConfigs().forEach((config) => config.markAsSaved());
  }

  /**
   * @param {function} release
   */

  #release(release) {
    try {
      release();
    } catch {
      // Releasing reports ERELEASED/ENOTACQUIRED when the lock was already
      // gone. Nothing thrown from here may escape - it would replace the
      // outcome the caller actually needs with a message about lock
      // bookkeeping. A lock that cannot be released goes stale and is reclaimed
      // by the next writer anyway.
    }
  }

  /**
   * Take the config file lock, waiting out a concurrent writer.
   *
   * proper-lockfile's sync API has no built-in retry, and failing on the first
   * contended attempt would surface a spurious error for a lock that is only
   * held for a rename.
   *
   * @returns {function} release
   */
  #acquireLock() {
    const deadline = Date.now() + this.lockAcquireTimeoutMs;

    for (;;) {
      try {
        return lockfile.lockSync(this.configFilePath, {
          lockfilePath: this.lockFilePath,
          // The config file legitimately does not exist on first run, and
          // realpath resolution would fail on it.
          realpath: false,
          stale: this.lockStaleMs,
          // Rethrowing here would kill the process - the handler runs from a
          // refresh timer, not this call stack. Recording it instead lets the
          // next save refuse, which matters because a command holding the lock
          // across its run relies on it for exclusivity.
          onCompromised: () => {
            this.#compromised = true;
          },
        });
      } catch (e) {
        if (e.code !== 'ELOCKED') {
          throw e;
        }

        if (Date.now() >= deadline) {
          throw new Error(`Timed out waiting to change '${this.configFilePath}'.`
            + ' Another dashmate command is modifying configuration - wait for it'
            + ' to finish and run this again.');
        }

        sleepSync(LOCK_RETRY_INTERVAL_MS);
      }
    }
  }
}
