import fs from 'fs';
import { Args } from '@oclif/core';
import BaseCommand from '../../oclif/command/BaseCommand.js';
import resolveConfigDirectory, { assertConfigNameAvailable } from '../../config/resolve-config-directory.js';

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
   * @param {ConfigFileJsonRepository} configFileRepository
   * @param {writeConfigTemplates} writeConfigTemplates
   * @param {HomeDir} homeDir
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
    homeDir,
  ) {
    assertConfigNameAvailable(configName);

    const serviceConfigsPath = resolveConfigDirectory(homeDir, configName);

    // Read, change and save in one locked step, so a config created here cannot
    // revert a change another command saved in the meantime.
    configFileRepository.update((updatedConfigFile) => {
      // A directory without a matching config belongs to an interrupted create
      // or remove and may contain private files that a new node must not adopt.
      if (!updatedConfigFile.isConfigExists(configName) && fs.existsSync(serviceConfigsPath)) {
        throw new Error(`Service files for '${configName}' already exist without a config.`
          + ` Run 'dashmate config remove ${configName}' to clean them up, then retry.`);
      }

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
