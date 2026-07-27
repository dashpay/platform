import { Args } from '@oclif/core';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import Config from '../../config/Config.js';
import InvalidOptionPathError from '../../config/errors/InvalidOptionPathError.js';

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
    // Validate the path against the schema, not against the currently-set
    // value. `config.get(...)` would throw `InvalidOptionPathError` for any
    // key inside a map-shaped property (e.g. `…buildArgs.SDK_TEST_DATA`)
    // because the value doesn't exist yet — that gate is the wrong shape for
    // schemas that use `additionalProperties: <schema>` to model open maps.
    // `Config.isSchemaPathAllowed` walks the schema and permits descent into
    // both typed `properties` and `additionalProperties` value schemas.
    if (!Config.isSchemaPathAllowed(optionPath)) {
      throw new InvalidOptionPathError(optionPath);
    }

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
