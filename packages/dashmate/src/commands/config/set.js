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
    configFileRepository,
    writeConfigTemplates,
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

    // Read, change and save in one locked step, against the config name resolved
    // for this command rather than re-resolving the default, which another
    // process may have changed. Mutating the copy loaded at startup and saving it
    // on exit would write a snapshot that is already out of date, reverting
    // anything saved in between.
    const configName = config.getName();

    configFileRepository.update((freshConfigFile) => {
      freshConfigFile.getConfig(configName).set(optionPath, value);
    }, {
      // Rendered inside the lock, so two commands changing the same config
      // cannot save in one order and render in the other.
      beforeSave: (freshConfigFile) => writeConfigTemplates(freshConfigFile.getConfig(configName)),
    });

    // eslint-disable-next-line no-console
    console.log(`${optionPath} set to ${optionValue}`);
  }
}
