import fs from 'fs';
import { Args } from '@oclif/core';
import BaseCommand from '../../oclif/command/BaseCommand.js';
import resolveConfigDirectory from '../../config/resolve-config-directory.js';

export default class ConfigRemoveCommand extends BaseCommand {
  static description = 'Remove config';

  static args = {
    config: Args.string(
      {
        name: 'config',
        required: true,
        description: 'config name', // only allow input to be from a discrete set
      },
    ),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @param {DefaultConfigs} defaultConfigs
   * @param {HomeDir} homeDir
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      config: configName,
    },
    flags,
    configFile,
    defaultConfigs,
    homeDir,
    configFileRepository,
  ) {
    if (defaultConfigs.has(configName)) {
      throw new Error(`system config ${configName} can't be removed.\nPlease use 'dashmate reset --hard --config=${configName}' command to reset the configuration`);
    }

    const serviceConfigsPath = resolveConfigDirectory(homeDir, configName);

    // An absent config is a cleanup retry after a previous post-save delete
    // failed. The create command rejects this directory until cleanup succeeds.
    configFileRepository.update((freshConfigFile) => {
      if (freshConfigFile.isConfigExists(configName)) {
        freshConfigFile.removeConfig(configName);
      }
    }, {
      onSaved: () => fs.rmSync(serviceConfigsPath, { recursive: true, force: true }),
    });

    // eslint-disable-next-line no-console
    console.log(`${configName} removed`);
  }
}
