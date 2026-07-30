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

    // Read, change and save in one locked step. Removing from the state loaded
    // at startup would revert anything another command saved in the meantime,
    // and removing a config another command already removed now fails here
    // instead of writing.
    configFileRepository.update((freshConfigFile) => {
      freshConfigFile.removeConfig(configName);
    });

    // Only once the removal is saved. Deleting first would leave the service
    // files gone while config.json still listed the config if saving failed.
    fs.rmSync(serviceConfigsPath, {
      recursive: true,
      force: true,
    });

    // eslint-disable-next-line no-console
    console.log(`${configName} removed`);
  }
}
