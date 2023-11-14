import { Args } from '@oclif/core';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';

export default class ConfigSetCommand extends ConfigBaseCommand {
  static description = `Set config option

Sets a configuration option in the default config
`;

  static flags = {
    ...ConfigBaseCommand.flags,
  };

  static args = {
    option: Args.string({
      name: 'option',
      required: true,
      description: 'option path',
    }),
    value: Args.string({
      name: 'value',
      required: true,
      description: 'the option value',
    }),
  };

  /**
   * @param args
   * @param flags
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      option: optionPath,
      value: optionValue,
    },
    flags,
    config,
  ) {
    // check for existence
    config.get(optionPath);

    let value;

    try {
      value = JSON.parse(optionValue);
    } catch (e) {
      value = optionValue;
    }

    config.set(optionPath, value);

    // eslint-disable-next-line no-console
    console.log(`${optionPath} set to ${optionValue}`);
  }
}
