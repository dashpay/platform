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
 * Comfortably longer than the milliseconds the lock is actually held, so a busy
 * event loop cannot get its live lock stolen, but short enough that a process
 * killed while holding it does not block the next command for long.
 */
const LOCK_STALE_MS = 10000;

/**
 * Waiting is synchronous, which means signal handlers do not run while it is in
 * progress. Kept just above the stale threshold - long enough to outlast a dead
 * holder's lock, short enough that Ctrl-C is never ignored for long.
 */
const LOCK_ACQUIRE_TIMEOUT_MS = LOCK_STALE_MS + 2000;

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
   * @param {migrateConfigFile} migrateConfigFile
   * @param {HomeDir} homeDir
   */
  constructor(migrateConfigFile, homeDir) {
    this.migrateConfigFile = migrateConfigFile;
    this.ajv = new Ajv();
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
    const release = this.#acquireLock();

    try {
      const configFile = this.read(options);

      mutate(configFile);

      this.#save(configFile);

      return configFile;
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
    const release = this.#acquireLock();

    try {
      this.#save(configFile);
    } finally {
      this.#release(release);
    }
  }

  /**
   * Serialize and replace the file, then mark the state clean.
   *
   * Caller must hold the lock.
   *
   * @param {ConfigFile} configFile
   */
  #save(configFile) {
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
    const deadline = Date.now() + LOCK_ACQUIRE_TIMEOUT_MS;

    for (;;) {
      try {
        return lockfile.lockSync(this.configFilePath, {
          lockfilePath: this.lockFilePath,
          // The config file legitimately does not exist on first run, and
          // realpath resolution would fail on it.
          realpath: false,
          stale: LOCK_STALE_MS,
          // The library's default handler rethrows from a refresh timer rather
          // than this call stack, which would take the whole process down. The
          // critical section is a read, a mutation and a rename, so it finishes
          // well inside the stale threshold and a compromised lock is not a
          // condition this can act on.
          onCompromised: () => {},
        });
      } catch (e) {
        if (e.code !== 'ELOCKED' || Date.now() >= deadline) {
          throw e;
        }

        sleepSync(LOCK_RETRY_INTERVAL_MS);
      }
    }
  }
}
