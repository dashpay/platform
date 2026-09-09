import fs from 'fs';
import Ajv from 'ajv';
import path from 'path';
import { randomUUID } from 'crypto';
import lockfile from 'proper-lockfile';
import semver from 'semver';
import writeFileAtomic from 'write-file-atomic';
import Config from '../Config.js';
import { PACKAGE_ROOT_DIR } from '../../constants.js';
import ConfigFileNotFoundError from '../errors/ConfigFileNotFoundError.js';
import InvalidConfigFileFormatError from '../errors/InvalidConfigFileFormatError.js';
import configFileJsonSchema from './configFileJsonSchema.js';
import ConfigFile from './ConfigFile.js';
import ConfigurationLockLostError from '../../ssl/errors/ConfigurationLockLostError.js';

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
   * Number of matching acquire() calls for the current process-held lease.
   *
   * @type {number}
   */
  #leaseDepth = 0;

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
   * @param {createConfigFile} createConfigFile
   * @param {string} configFormatVersion - the format this build produces, used
   *   to tell a migrating read from a clean one without running any migration
   * @param {Object} [configFileLockOptions={}] - lock timing overrides
   * @param {number} [configFileLockOptions.stale]
   * @param {number} [configFileLockOptions.acquireTimeout]
   */
  constructor(
    migrateConfigFile,
    homeDir,
    createConfigFile,
    configFormatVersion,
    configFileLockOptions = {},
  ) {
    this.migrateConfigFile = migrateConfigFile;
    this.configFormatVersion = configFormatVersion;
    this.createConfigFile = createConfigFile;
    this.ajv = new Ajv();
    this.lockStaleMs = configFileLockOptions.stale ?? LOCK_STALE_MS;
    this.lockAcquireTimeoutMs = configFileLockOptions.acquireTimeout ?? LOCK_ACQUIRE_TIMEOUT_MS;
    this.homeDirPath = homeDir.getPath();
    this.configFilePath = homeDir.joinPath('config.json');
    // Locking a sibling rather than the config file itself keeps first run
    // working, where there is no config file to lock yet.
    this.lockFilePath = homeDir.joinPath('.config.json.lock');
  }

  /**
   * Load configs from file
   *
   * @param {Object} [options={}]
   * @param {boolean|function(Object): boolean} [options.skipValidation=false]
   *   Skip per-config schema validation globally or for configs selected by a predicate
   * @returns {ConfigFile}
   */
  read(options = {}) {
    return this.#buildConfigFile(this.#readRawConfigFile(), options);
  }

  /**
   * The bytes on disk, parsed, and nothing more.
   *
   * Separate from building a ConfigFile so a caller that has to decide
   * something about the file before migrating it can decide and migrate from
   * the same snapshot. Reading twice leaves a window in which the file can be
   * replaced between the decision and the work it authorised.
   *
   * @returns {Object}
   */
  #readRawConfigFile() {
    if (!fs.existsSync(this.configFilePath)) {
      throw new ConfigFileNotFoundError(this.configFilePath);
    }

    const configFileJSON = fs.readFileSync(this.configFilePath, 'utf8');

    try {
      return JSON.parse(configFileJSON);
    } catch (e) {
      throw new InvalidConfigFileFormatError(this.configFilePath, e);
    }
  }

  /**
   * @param {Object} configFileData - already read and parsed
   * @param {Object} [options={}]
   * @returns {ConfigFile}
   */
  #buildConfigFile(configFileData, options = {}) {
    const { skipValidation = false } = options;

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
        .map(([name, opts]) => {
          const isValidationSkipped = typeof skipValidation === 'function'
            ? skipValidation({
              name,
              options: opts,
              configFileData: migratedConfigFileData,
            })
            : skipValidation;

          return new Config(name, opts, isValidationSkipped);
        });
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
   * both survive. The locked read ensures the mutator never receives a stale
   * snapshot from an earlier operation.
   *
   * The lock is held for a read, a mutation and a rename - milliseconds - so it
   * is not a meaningful serialization point for commands that only edit config.
   *
   * @param {function(ConfigFile): void} mutate
   * @param {Object} [options={}]
   * @param {function(ConfigFile): void} [options.onSaved] - runs after the save
   *   and before the lock is released
   * @returns {void}
   */
  update(mutate, { onSaved } = {}) {
    this.#locked(() => {
      // First run has nothing to read yet, and creating the defaults here rather
      // than separately keeps that on the same locked path as every other change
      // - two nodes being set up at once cannot both create the file.
      const configFile = fs.existsSync(this.configFilePath)
        ? this.read()
        : this.createConfigFile();

      mutate(configFile);

      this.#save(configFile);

      if (!this.isExclusive()) {
        throw new ConfigurationLockLostError('Lost the configuration lock after saving the config file;'
          + ' follow-up filesystem changes were not run. Re-run the command.');
      }

      if (onSaved) {
        onSaved(configFile);
      }
    });
  }

  /**
   * Read the config file, saving the result when reading migrated it.
   *
   * Migrating produces a new shape that has to reach disk. Doing both under one
   * lock keeps it from reverting anything saved between the read and the save.
   *
   * Clean reads do not acquire the lock. When the first read discovers a
   * migration, the file is read again and migrated under the lock before it is
   * saved. This keeps ordinary read-only commands available during long-running
   * configuration changes without letting a migrated snapshot overwrite a newer
   * write.
   *
   * @param {Object} [options={}] - passed through to read()
   * @param {function(Config[]): void} [onMigrated] - runs before the migrated
   *   config file is saved and while the lock is held
   * @returns {{configFile: ConfigFile}}
   */
  readAndMigrate(options = {}, onMigrated = undefined) {
    const readResult = () => {
      const configFile = this.read(options);
      const migrated = configFile.getAllConfigs().filter((config) => config.isChanged());

      return { configFile, migrated };
    };

    // Decide whether a migration is due from the recorded version alone.
    // Migrations are not all pure - some move service files on disk and delete
    // the originals - so running them to find out would do that work outside
    // the lock, and again inside it.
    if (!this.#isMigrationDue()) {
      return { configFile: this.read(options) };
    }

    return this.#locked(() => {
      if (!this.isExclusive()) {
        throw new ConfigurationLockLostError('Lost the configuration lock before the config file was migrated.');
      }

      // Another process may have migrated or changed the file while this
      // process waited, so never save the result of the unlocked probe.
      const result = readResult();

      const { configFile, migrated } = result;

      if (configFile.isChanged()) {
        if (onMigrated) {
          onMigrated(migrated);
        }

        this.#save(configFile);
      }

      return { configFile };
    });
  }

  /**
   * Whether the file on disk records an older format than this build produces.
   *
   * Reads the recorded version only. Running the migrations to find out would
   * perform their side effects - the 0.25.7 migration moves TLS files and
   * deletes the originals - before this process holds the lock.
   *
   * @returns {boolean}
   */
  #isMigrationDue() {
    // Without a target to compare against there is no way to tell, and guessing
    // wrong means running a migration's file moves outside the lock. Taking the
    // lock costs a reader nothing it would not already pay when a migration is
    // genuinely due.
    if (typeof this.configFormatVersion !== 'string') {
      return true;
    }

    let recordedVersion;

    try {
      recordedVersion = JSON.parse(
        fs.readFileSync(this.configFilePath, 'utf8'),
      ).configFormatVersion;
    } catch {
      // An unreadable or malformed file is read()'s to report, with the error
      // that names the file and the reason.
      return true;
    }

    if (typeof recordedVersion !== 'string' || semver.valid(recordedVersion) === null) {
      return true;
    }

    return semver.lt(recordedVersion, this.configFormatVersion);
  }

  /**
   * Whether this process still holds the lock it took.
   *
   * A caller with a side effect of its own to run - rendering service files -
   * has to know before running it, not after. Losing the lock means another
   * process may already have saved and rendered newer state, and writing over
   * that from a stale copy is the lost update this exists to prevent.
   *
   * @returns {boolean}
   */
  isExclusive() {
    return !this.#compromised;
  }

  /**
   * Hold the lock for as long as this process is changing configuration.
   *
   * For a command that reconfigures a node over minutes, taking the lock before
   * it reads is what makes its state current: there is no window between reading
   * and saving for another process to write into. Short changes should use
   * update() instead - this blocks every other writer until release() is called.
   *
   * Every acquire adds a lease level that needs a matching release. Locked
   * repository operations reuse the held lease without adding another level.
   *
   * @returns {void}
   */
  acquire() {
    if (this.#heldRelease !== null) {
      this.#leaseDepth += 1;
      return;
    }

    this.#compromised = false;
    this.#heldRelease = this.#acquireLock();
    this.#leaseDepth = 1;
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

    if (this.#leaseDepth > 1) {
      this.#leaseDepth -= 1;
      return;
    }

    const release = this.#heldRelease;

    this.#heldRelease = null;
    this.#leaseDepth = 0;

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

    // A compromise belongs to the lock that was lost. A later independent
    // acquisition restores exclusivity and must not inherit that failure.
    this.#compromised = false;
    const release = this.#acquireLock();
    this.#heldRelease = release;
    this.#leaseDepth = 1;

    try {
      return fn();
    } finally {
      if (this.#heldRelease === release) {
        this.release();
      }
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
   * Serialize and replace the file, then mark the config collection saved.
   *
   * Individual configs remain changed until their service templates are
   * rendered. A mid-command save may persist JSON before those files should take
   * effect, so clearing their flags here would make successful command
   * finalization skip the render.
   *
   * Caller must hold the lock.
   *
   * @param {ConfigFile} configFile
   */
  #save(configFile) {
    const configFileJSON = `${JSON.stringify(configFile.toObject(), undefined, 2)}\n`;

    if (this.#compromised) {
      // One path per rescue. A later command that also loses its lease must not
      // replace a copy nobody has looked at yet - it may be the only record of
      // an operator key typed into a setup that already registered a masternode
      // on chain.
      const rescuePath = path.join(this.homeDirPath, `.config.json.rescue-${randomUUID()}`);

      try {
        writeFileAtomic.sync(rescuePath, configFileJSON, {
          encoding: 'utf8',
          mode: 0o600,
        });
        fs.chmodSync(rescuePath, 0o600);
      } catch (e) {
        throw new Error(
          `Lost the lock on '${this.configFilePath}' while changing it,`
            + ` and could not preserve the pending configuration at '${rescuePath}': ${e.message}`,
          { cause: e },
        );
      }

      throw new Error(`Lost the lock on '${this.configFilePath}' while changing it,`
        + ` so saving now could overwrite another process. The pending configuration was saved`
        + ` to '${rescuePath}'. Review it before re-running the command.`);
    }

    writeFileAtomic.sync(this.configFilePath, configFileJSON, 'utf8');

    // Only now is the state actually on disk. Clearing these beforehand would
    // leave the config file claiming to be saved after a failed write.
    configFile.markAsSaved();
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
          throw new ConfigurationLockLostError(`Timed out waiting for configuration lock '${this.lockFilePath}'.`
            + ' It may be held by a Dashmate command, the dashmate helper during certificate'
            + ' renewal, or a running reindex. An abandoned lock after SIGKILL or power loss'
            + ' clears itself after about a minute; do not remove it manually while another'
            + ' process may still be running.');
        }

        sleepSync(LOCK_RETRY_INTERVAL_MS);
      }
    }
  }
}
