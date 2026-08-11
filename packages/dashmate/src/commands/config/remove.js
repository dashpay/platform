import fs from 'fs';
import { Args } from '@oclif/core';
import BaseCommand from '../../oclif/command/BaseCommand.js';
import resolveConfigDirectory, { isConfigNameAvailable } from '../../config/resolve-config-directory.js';

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

    // A name Dashmate owns resolves to one of its own files rather than to a
    // service directory - `config.json` resolves to the config file itself.
    // Deleting that would take every configuration with it, so the directory is
    // never touched for such a name. An entry left under one by an older
    // version is still removable: the listing goes, the file stays.
    const isRepositoryOwnedPath = !isConfigNameAvailable(configName);

    // An absent config is a cleanup retry after a previous post-save delete
    // failed. The create command rejects this directory until cleanup succeeds.
    let wasListed = false;

    configFileRepository.update((freshConfigFile) => {
      wasListed = freshConfigFile.isConfigExists(configName);

      if (wasListed) {
        freshConfigFile.removeConfig(configName);
      } else if (isRepositoryOwnedPath) {
        throw new Error(`'${configName}' is a name Dashmate reserves for its own files,`
          + ' and there is no config listed under it to remove.');
      }
    }, {
      onSaved: () => {
        if (!isRepositoryOwnedPath) {
          fs.rmSync(serviceConfigsPath, { recursive: true, force: true });
        }
      },
    });

    // eslint-disable-next-line no-console
    console.log(`${configName} removed`);
  }
}
