import { asValue } from 'awilix';
import { Flags } from '@oclif/core';
import BaseCommand from './BaseCommand.js';

/**
 * @abstract
 */
export default class ConfigBaseCommand extends BaseCommand {
  static flags = {
    config: Flags.string({
      description: 'configuration name to use',
      default: null,
    }),
    ...BaseCommand.flags,
  };

  async run() {
    return super.run();
  }
}
