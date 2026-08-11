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

    // Deleting the directory outright has no safe moment. Before the save, a
    // failed save leaves a config listed with its files gone; after it, a failed
    // delete leaves files behind under a name that is now free to re-create -
    // including the previous node's TLS private key, which a new config of the
    // same name would inherit. Moving it aside first separates the two: the move
    // is reversible, and what is left behind is named so it cannot be mistaken
    // for a live config.
    const tombstonePath = `${serviceConfigsPath}.removed-${process.pid}`;

    try {
      // Read, change and save in one locked step. Removing from the state loaded
      // at startup would revert anything another command saved in the meantime,
      // and removing a config another command already removed now fails here
      // instead of writing.
      configFileRepository.update((freshConfigFile) => {
        freshConfigFile.removeConfig(configName);
      }, {
        // Under the lock, so a concurrent re-creation cannot write into the
        // directory between the move and the save. Guarded on existence so
        // running it again after a failed save is a no-op.
        beforeSave: () => {
          if (fs.existsSync(serviceConfigsPath)) {
            fs.renameSync(serviceConfigsPath, tombstonePath);
          }
        },
        // Only once the removal is durable. A failure here leaves the tombstone
        // rather than a directory under a re-creatable name.
        onSaved: () => fs.rmSync(tombstonePath, { recursive: true, force: true }),
      });
    } catch (e) {
      // Which failure this was decides what to do with the moved directory. If
      // the removal never reached disk the config is still listed and needs its
      // files back. If it did, and only the delete failed, putting them back
      // would restore them under a name that is now free to re-create - so the
      // tombstone stays, out of reach of a new config of the same name.
      let isStillListed = false;

      try {
        isStillListed = configFileRepository.read().isConfigExists(configName);
      } catch {
        // Unable to tell. Leaving the tombstone is the recoverable direction:
        // an operator can move it back, where a wrong restore would quietly
        // hand the previous node's keys to the next config of this name.
      }

      if (isStillListed && fs.existsSync(tombstonePath) && !fs.existsSync(serviceConfigsPath)) {
        fs.renameSync(tombstonePath, serviceConfigsPath);
      }

      throw e;
    }

    // eslint-disable-next-line no-console
    console.log(`${configName} removed`);
  }
}
