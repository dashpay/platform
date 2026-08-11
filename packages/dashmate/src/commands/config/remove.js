import fs from 'fs';
import { Args } from '@oclif/core';
import BaseCommand from '../../oclif/command/BaseCommand.js';
import resolveConfigDirectory, { isConfigNameAvailable } from '../../config/resolve-config-directory.js';

/**
 * Whether two paths are the same file on this filesystem.
 *
 * @param {string} left
 * @param {string} right
 * @return {boolean}
 */
function isSameFile(left, right) {
  try {
    const a = fs.statSync(left);
    const b = fs.statSync(right);

    return a.dev === b.dev && a.ino === b.ino;
  } catch {
    // One of them is not there, so nothing can be destroyed by mistake.
    return false;
  }
}

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

    // A name Dashmate owns can resolve to one of its own files rather than to a
    // service directory - `config.json` resolves to the config file itself, and
    // deleting that takes every configuration with it. Whether it actually does
    // depends on the filesystem: `CONFIG.JSON` is the same file on macOS and a
    // separate directory on Linux, and that separate directory holds TLS keys
    // and a dash.conf carrying masternode and spork private keys. So compare
    // the resolved paths rather than trusting the name.
    const isRepositoryOwnedPath = !isConfigNameAvailable(configName)
      && isSameFile(serviceConfigsPath, homeDir.joinPath('config.json'));

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
