const { Command } = require('@oclif/command');

const graceful = require('node-graceful');

const getFunctionParams = require('../../util/getFunctionParams');

const createDIContainer = require('../../createDIContainer');

/**
 * @abstract
 */
class BaseCommand extends Command {
  async init() {
    this.container = await createDIContainer();

    const stopAllContainers = this.container.resolve('stopAllContainers');
    const startedContainers = this.container.resolve('startedContainers');

    // graceful exit
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

    const { args, flags } = this.parse(this.constructor);

    return this.runWithDependencies(args, flags, ...dependencies);
  }

  async finally(err) {
    const stopAllContainers = this.container.resolve('stopAllContainers');
    const startedContainers = this.container.resolve('startedContainers');

    await stopAllContainers(startedContainers.getContainers());

    return super.finally(err);
  }
}

module.exports = BaseCommand;
