import {BaseCommand} from "../../oclif/command/BaseCommand.js";

export class ConfigDefaultCommand extends BaseCommand {
  static description = `Manage default config

Shows default config name or sets another config as default
`;


  static args = [{
    name: 'config',
    required: false,
    description: 'config name',
    default: null,
  }];


  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      config: configName,
    },
    flags,
    configFile,
  ) {
    if (configName === null) {
      // eslint-disable-next-line no-console
      console.log(configFile.getDefaultConfigName());
    } else {
      configFile.setDefaultConfigName(configName);

      // eslint-disable-next-line no-console
      console.log(`${configName} config set as default`);
    }
  }
}
