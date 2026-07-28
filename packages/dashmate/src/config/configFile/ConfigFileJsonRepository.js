import fs from 'fs';
import Ajv from 'ajv';
import path from 'path';
import lockfile from 'proper-lockfile';
import writeFileAtomic from 'write-file-atomic';
import Config from '../Config.js';
import { PACKAGE_ROOT_DIR } from '../../constants.js';
import ConfigFileNotFoundError from '../errors/ConfigFileNotFoundError.js';
import InvalidConfigFileFormatError from '../errors/InvalidConfigFileFormatError.js';
import ConfigFileConflictError from '../errors/ConfigFileConflictError.js';
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
   * What this instance last observed on disk:
   *
   * - `undefined` - never looked, so nothing can be claimed about the file
   * - `null`      - looked, and there was no file
   * - `string`    - the exact bytes that were there
   *
   * "Never looked" and "looked and found nothing" have to stay distinct, or two
   * nodes being set up concurrently would both write over each other believing
   * they were creating the file.
   *
   * Concurrent writers are detected by comparing against this, so it must be
   * refreshed on every successful write - the dashmate helper reads once at
   * startup and then writes on every certificate renewal for the life of the
   * process, and would otherwise conflict with its own previous write.
   *
   * One instance tracks one config file lifecycle: a second read() rebaselines
   * it, so do not write a ConfigFile obtained from an earlier read afterwards.
   *
   * @type {string|null|undefined}
   */
  #baseline;

  /**
   * Distinguishes parked snapshots written by this process within the same
   * millisecond.
   *
   * @type {number}
   */
  #rejectedCount = 0;

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
      // Record that the file was observed absent. The caller creates a default
      // config file from here, and that write must still lose to another
      // process that created one first.
      this.#baseline = null;

      throw new ConfigFileNotFoundError(this.configFilePath);
    }

    const configFileJSON = fs.readFileSync(this.configFilePath, 'utf8');

    this.#baseline = configFileJSON;

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
   * Save configs to file
   *
   * Refuses to write when the file changed on disk since this instance read it,
   * rather than reverting whatever the other process saved. The check and the
   * replacement happen under a lock so two writers cannot both pass the check
   * and then both write; the lock is held only for that, never for the duration
   * of a command.
   *
   * @param {ConfigFile} configFile
   * @throws {ConfigFileConflictError} when another process wrote first
   * @returns {void}
   */
  write(configFile) {
    const configFileJSON = `${JSON.stringify(configFile.toObject(), undefined, 2)}\n`;

    const release = this.#acquireLock();

    try {
      const currentJSON = fs.existsSync(this.configFilePath)
        ? fs.readFileSync(this.configFilePath, 'utf8')
        : null;

      // Any observed change conflicts, including present-to-absent: recreating
      // a config file somebody deliberately removed is the same lost update in
      // the other direction.
      if (this.#baseline !== undefined && currentJSON !== this.#baseline) {
        throw this.#rejectStaleWrite(configFileJSON);
      }

      writeFileAtomic.sync(this.configFilePath, configFileJSON, 'utf8');

      this.#baseline = configFileJSON;

      // Only now is the state actually on disk. Clearing these before the write
      // would leave the configs claiming to be saved after a failed one.
      configFile.markAsSaved();
      configFile.getAllConfigs().forEach((config) => config.markAsSaved());
    } finally {
      try {
        release();
      } catch {
        // Releasing reports ERELEASED/ENOTACQUIRED when the lock was already
        // gone. Nothing thrown from here may escape: it would replace the
        // outcome the caller actually needs - including the conflict error
        // naming where their configuration was parked - with a message about
        // lock bookkeeping. A lock that cannot be released goes stale and is
        // reclaimed by the next writer anyway.
      }
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
          // The library's default handler rethrows, and it runs from a refresh
          // timer rather than this call stack, so a lock compromised mid-write
          // would take the whole process down. There is nothing useful to do
          // about it here: the critical section lasts milliseconds, and the
          // byte comparison against the baseline is what actually prevents a
          // lost update - the lock only narrows the window it runs in.
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

  /**
   * Park the state we refused to write, so material generated during the
   * command is recoverable, and describe where it went.
   *
   * @param {string} configFileJSON
   * @returns {ConfigFileConflictError}
   */
  #rejectStaleWrite(configFileJSON) {
    // Colons are not legal in Windows filenames. The pid separates concurrent
    // processes and the counter separates two conflicts from this one inside
    // the same millisecond, so no parked state is ever overwritten by another.
    const stamp = new Date().toISOString().replace(/[:.]/g, '-');

    this.#rejectedCount += 1;

    const rejectedPath = `${this.configFilePath}.rejected-${stamp}-${process.pid}-${this.#rejectedCount}`;

    try {
      writeFileAtomic.sync(rejectedPath, configFileJSON, { encoding: 'utf8', mode: 0o600 });
    } catch (e) {
      return new ConfigFileConflictError(this.configFilePath, null, e);
    }

    return new ConfigFileConflictError(this.configFilePath, rejectedPath);
  }
}
