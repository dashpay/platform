import fs from 'fs';
import {BaseCommand} from "../../oclif/command/BaseCommand.js";

export class ConfigRemoveCommand extends BaseCommand {
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

ConfigRemoveCommand.description = 'Remove config';

ConfigRemoveCommand.args = [{
  name: 'config',
  required: true,
  description: 'config name',
}];
