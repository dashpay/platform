import { Command, Flags, settings } from '@oclif/core';

import { asValue } from 'awilix';

import graceful from 'node-graceful';

import dotenv from 'dotenv';
import createDIContainer from '../../createDIContainer.js';
import ConfigFileNotFoundError from '../../config/errors/ConfigFileNotFoundError.js';
import getFunctionParams from '../../util/getFunctionParams.js';

/**
 * @abstract
 */
export default class BaseCommand extends Command {
  static flags = {
    verbose: Flags.boolean({
      char: 'v',
      description: 'use verbose mode for output',
      default: false,
    }),
  };

  /**
   * Whether this run changes nothing on disk. A command that reconfigures a
   * node can still have a mode that only reports, and such a mode has to keep
   * that promise all the way down: it takes no lock, saves no configuration,
   * and does not persist a migration it happened to need.
   *
   * Set from the command's flags in init(). A command that declares one of
   * these modes gives up its end-of-run save in that mode, which is the point.
   */
  isReadOnlyRun = false;

  /**
   * Whether this run holds the configuration lock. Defaults to what the command
   * declares and is narrowed once its flags are known.
   */
  holdsConfigLock = this.constructor.mutatesConfig === true;

  /**
   * @param {Object} options
   * @return {Promise<AwilixContainer>}
   */
  async createContainer(options) {
    return createDIContainer(options);
  }

  async init() {
    // Read environment variables from .env file
    dotenv.config();

    const { args, flags } = await this.parse(this.constructor);

    this.parsedArgs = args;
    this.parsedFlags = flags;

    this.container = await this.createContainer(process.env);

    // Load configs
    /**
     * @type {ConfigFileJsonRepository}
     */
    const configFileRepository = this.container.resolve('configFileRepository');

    // A command that reconfigures a node changes config repeatedly while doing
    // long work, so it takes the lock before reading: its state is then current
    // for the whole run and no other writer can get in between. Everything else
    // changes config through configFileRepository.update() and needs nothing
    // here.
    //
    // Such a command may still have a mode that changes nothing - a read-only
    // preflight, say - and taking a write lock there would let it fail on a
    // lock timeout for no reason, so it can opt that mode out. The migration
    // below is opted out with it: migrating writes and renders under the same
    // lock, and it is due on exactly the run right after an upgrade.
    this.isReadOnlyRun = this.constructor.isReadOnlyRun?.(this.parsedFlags) === true;
    this.holdsConfigLock = this.holdsConfigLock && !this.isReadOnlyRun;

    if (this.holdsConfigLock) {
      configFileRepository.acquire();
    }

    let configFile;
    try {
      // Load config collection from config file, saving it again if loading
      // migrated it - under one lock, so the migrated shape cannot revert a
      // change saved in between.
      const skipValidation = this.constructor.shouldSkipConfigValidation?.(
        this.parsedFlags,
      ) ?? false;

      ({ configFile } = configFileRepository.readAndMigrate(
        {
          skipValidation,
          readOnly: this.isReadOnlyRun,
        },
        (migratedConfigs) => {
          const writeConfigTemplates = this.container.resolve('writeConfigTemplates');

          migratedConfigs.forEach(writeConfigTemplates);
        },
      ));
    } catch (e) {
      // Create default config collection if config file is not present
      // on the first start for example

      if (!(e instanceof ConfigFileNotFoundError)) {
        throw e;
      }

      /**
       * @type {createConfigFile}
       */
      const createConfigFile = this.container.resolve('createConfigFile');

      configFile = createConfigFile();
    }

    // Register config collection in the container
    this.container.register({
      configFile: asValue(configFile),
    });

    // Graceful exit
    const stopAllContainers = this.container.resolve('stopAllContainers');
    const startedContainers = this.container.resolve('startedContainers');

    graceful.exitOnDouble = false;
    graceful.on('exit', async () => {
      // remove all attached listeners from other libraries to mute there output
      process.removeAllListeners('uncaughtException');
      process.removeAllListeners('unhandledRejection');

      process.on('unhandledRejection', () => {});
      process.on('uncaughtException', () => {});

      // stop and remove all started containers
      await stopAllContainers(startedContainers.getContainers());
    });
  }

  async run() {
    if (!this.runWithDependencies) {
      throw new Error('`run` or `runWithDependencies` must be implemented');
    }

    const params = getFunctionParams(this.runWithDependencies, 2);

    const dependencies = params.map((paramName) => this.container.resolve(paramName));

    return this.runWithDependencies(this.parsedArgs, this.parsedFlags, ...dependencies);
  }

  async finally(err) {
    try {
      await this.saveConfigAndStopContainers(err);
    } finally {
      // Whether the command succeeded, failed, or failed before it started, the
      // lock must not outlive it.
      if (this.container && this.holdsConfigLock) {
        this.container.resolve('configFileRepository').release();
      }
    }

    return super.finally(err);
  }

  /**
   * @param {Error|undefined} err
   */
  async saveConfigAndStopContainers(err) {
    // Save configs collection
    if (this.container) {
      let saveError;

      /**
       * @var {ConfigFileJsonRepository} configFileRepository
       */
      const configFileRepository = this.container.resolve('configFileRepository');

      try {
        // Only a command that held the lock for its whole run may save the config
        // file it loaded - it read inside the lock, so its state is current. Any
        // other command changes configuration through update(), and saving its
        // startup copy here would write a snapshot from before the command ran.
        if (this.holdsConfigLock
          && this.container.has('configFile') && err === undefined) {
          /**
           * @var {ConfigFile} configFile
           */
          const configFile = this.container.resolve('configFile');

          if (configFile.isChanged()) {
            // Rendering must not start once the lock is gone: another process
            // may already have saved and rendered newer state, and these files
            // would overwrite it from a snapshot taken before it existed.
            if (!configFileRepository.isExclusive()) {
              // Saving refuses too, but on the way it writes what this command
              // produced to a rescue file - which for setup or a reindex is the
              // only copy of work that already happened out in the world. Going
              // through it rather than throwing here keeps that, and it still
              // stops before anything is rendered.
              configFileRepository.write(configFile);
            }

            const changedConfigs = configFile.getAllConfigs()
              .filter((config) => config.isChanged());

            /**
             * @var {writeConfigTemplates} writeConfigTemplates
             */
            const writeConfigTemplates = this.container.resolve('writeConfigTemplates');

            // JSON is authoritative. If rendering fails or the process is killed
            // next, an explicit config render repairs the stale service files.
            configFileRepository.write(configFile);

            changedConfigs.forEach(writeConfigTemplates);
          }
        }
      } catch (e) {
        saveError = e;
      }

      // Stop all running containers
      const stopAllContainers = this.container.resolve('stopAllContainers');
      const startedContainers = this.container.resolve('startedContainers');

      try {
        await stopAllContainers(
          startedContainers.getContainers(),
          {
            remove: !settings.debug,
          },
        );
      } catch (cleanupError) {
        if (!saveError) {
          throw cleanupError;
        }

        saveError.cleanupError = cleanupError;
      }

      if (saveError) {
        throw saveError;
      }
    }
  }
}
