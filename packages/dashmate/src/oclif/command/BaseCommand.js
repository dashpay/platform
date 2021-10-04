const { Command, flags: flagTypes } = require('@oclif/command');

const { asValue } = require('awilix');

const graceful = require('node-graceful');

const dotenv = require('dotenv');

const getFunctionParams = require('../../util/getFunctionParams');

const createDIContainer = require('../../createDIContainer');

const ConfigFileNotFoundError = require('../../config/errors/ConfigFileNotFoundError');

/**
 * @abstract
 */
class BaseCommand extends Command {
  async init() {
    // Read environment variables from .env file
    dotenv.config();

    const { args, flags } = this.parse(this.constructor);

    this.parsedArgs = args;
    this.parsedFlags = flags;

    this.container = await createDIContainer();

    // Set up home dir
    /**
     * @type {ensureHomeDir}
     */
    const ensureHomeDir = this.container.resolve('ensureHomeDir');

    ensureHomeDir();

    // Load configs
    /**
     * @type {ConfigFileJsonRepository}
     */
    const configFileRepository = this.container.resolve('configFileRepository');

    let configFile;
    try {
      // Load config collection from config file
      configFile = await configFileRepository.read();
    } catch (e) {
      // Create default config collection if config file is not present
      // on the first start for example

      if (!(e instanceof ConfigFileNotFoundError)) {
        throw e;
      }

      /**
       * @type {createSystemConfigs}
       */
      const createSystemConfigs = this.container.resolve('createSystemConfigs');

      configFile = createSystemConfigs();
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
    // Save configs collection
    if (this.container) {
      const configFileRepository = this.container.resolve('configFileRepository');

      if (this.container.has('configFile')) {
        const configFile = this.container.resolve('configFile');

        await configFileRepository.write(configFile);
      }

      // Stop all running containers
      const stopAllContainers = this.container.resolve('stopAllContainers');
      const startedContainers = this.container.resolve('startedContainers');

      await stopAllContainers(
        startedContainers.getContainers(),
        {
          remove: !this.config.debug,
        },
      );
    }

    return super.finally(err);
  }
}

BaseCommand.flags = {
  verbose: flagTypes.boolean({
    char: 'v',
    description: 'use verbose mode for output',
    default: false,
  }),
};

module.exports = BaseCommand;
