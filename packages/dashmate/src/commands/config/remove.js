import fs from 'fs';
import { Args } from '@oclif/core';
import BaseCommand from '../../oclif/command/BaseCommand.js';

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
  ) {
    if (defaultConfigs.has(configName)) {
      throw new Error(`system config ${configName} can't be removed.\nPlease use 'dashmate reset --hard --config=${configName}' command to reset the configuration`);
    }

    configFile.removeConfig(configName);

    const serviceConfigsPath = homeDir.joinPath(configName);

    fs.rmSync(serviceConfigsPath, {
      recursive: true,
      force: true,
    });

    // eslint-disable-next-line no-console
    console.log(`${configName} removed`);
  }
}
