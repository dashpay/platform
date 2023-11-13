const { Command, Flags, settings } = require('@oclif/core');

const { default: loadWasmDpp } = require('@dashevo/wasm-dpp');

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
    // Load wasm-dpp for further usage
    await loadWasmDpp();

    // Read environment variables from .env file
    dotenv.config();

    const { args, flags } = await this.parse(this.constructor);

    this.parsedArgs = args;
    this.parsedFlags = flags;

    this.container = await createDIContainer(process.env);

    // Load configs
    /**
     * @type {ConfigFileJsonRepository}
     */
    const configFileRepository = this.container.resolve('configFileRepository');

    let configFile;
    try {
      // Load config collection from config file
      configFile = configFileRepository.read();
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
    // Save configs collection
    if (this.container) {
      /**
       * @var {ConfigFileJsonRepository} configFileRepository
       */
      const configFileRepository = this.container.resolve('configFileRepository');

      if (this.container.has('configFile') && err === undefined) {
        /**
         * @var {ConfigFile} configFile
         */
        const configFile = this.container.resolve('configFile');

        if (configFile.isChanged()) {
          configFileRepository.write(configFile);

          /**
           * @var {writeConfigTemplates} writeConfigTemplates
           */
          const writeConfigTemplates = this.container.resolve('writeConfigTemplates');

          configFile.getAllConfigs()
            .filter((config) => config.isChanged())
            .forEach(writeConfigTemplates);
        }
      }

      // Stop all running containers
      const stopAllContainers = this.container.resolve('stopAllContainers');
      const startedContainers = this.container.resolve('startedContainers');

      await stopAllContainers(
        startedContainers.getContainers(),
        {
          remove: !settings.debug,
        },
      );
    }

    return super.finally(err);
  }
}

BaseCommand.flags = {
  verbose: Flags.boolean({
    char: 'v',
    description: 'use verbose mode for output',
    default: false,
  }),
};

module.exports = BaseCommand;
