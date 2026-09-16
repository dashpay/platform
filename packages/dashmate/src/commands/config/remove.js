import fs from 'fs';
import { Args } from '@oclif/core';
import BaseCommand from '../../oclif/command/BaseCommand.js';
import resolveConfigDirectory, {
  getPortableConfigName,
  isConfigNameAvailable,
} from '../../config/resolve-config-directory.js';

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

    configFileRepository.update((freshConfigFile) => {
      freshConfigFile.removeConfig(configName);
    }, {
      onSaved: (freshConfigFile) => {
        // Old config files may contain names that are now reserved. Their JSON
        // entries remain removable, but their paths are never deletion targets.
        // They may also contain names that alias on portable filesystems. Keep
        // a shared directory while any exact JSON entry can still refer to it.
        const portableConfigName = getPortableConfigName(configName);
        const isDirectoryStillReferenced = freshConfigFile.getAllConfigs()
          .some((config) => getPortableConfigName(config.getName()) === portableConfigName);

        if (isConfigNameAvailable(configName) && !isDirectoryStillReferenced) {
          fs.rmSync(serviceConfigsPath, { recursive: true, force: true });
        }
      },
    });

    // eslint-disable-next-line no-console
    console.log(`${configName} removed`);
  }
}
