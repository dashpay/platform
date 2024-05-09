import { Command, Flags, settings } from '@oclif/core';

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

  async init() {
    console.log('init BaseCommand')
    // Read environment variables from .env file
    dotenv.config();

    const { args, flags } = await this.parse(this.constructor);

    this.parsedArgs = args;
    this.parsedFlags = flags;

    // Load configs
    /**
     * @type {ConfigFileJsonRepository}
     */

    // Graceful exit
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


    return this.runWithDependencies(this.parsedArgs, this.parsedFlags);
  }

  async finally(err) {
    // Save configs collection

    return super.finally(err);
  }
}
