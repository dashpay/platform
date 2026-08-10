import { Args } from '@oclif/core';
import BaseCommand from '../../oclif/command/BaseCommand.js';

export default class ConfigCreateCommand extends BaseCommand {
  static description = 'Create new config';

  static args = {
    config: Args.string({
      name: 'config',
      required: true,
      description: 'config name',
    }),
    from: Args.string({
      name: 'from',
      required: false,
      description: 'base new config on existing config',
      default: 'base',
    }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      config: configName,
      from: fromConfigName,
    },
    flags,
    configFile,
    configFileRepository,
    writeConfigTemplates,
  ) {
    // Read, change and save in one locked step, so a config created here cannot
    // revert a change another command saved in the meantime.
    configFileRepository.update((updatedConfigFile) => {
      updatedConfigFile.createConfig(configName, fromConfigName);
    }, {
      // The new config needs its service files, and rendering them inside the
      // lock keeps them consistent with what was saved.
      beforeSave: (freshConfigFile) => writeConfigTemplates(freshConfigFile.getConfig(configName)),
    });

    // eslint-disable-next-line no-console
    console.log(`${configName} created`);
  }
}
