import {asValue} from 'awilix';
import {Flags} from "@oclif/core";
import {ConfigIsNotPresentError} from "../../config/errors/ConfigIsNotPresentError.js";
import {BaseCommand} from "./BaseCommand.js";

/**
 * @abstract
 */
export class ConfigBaseCommand extends BaseCommand {
  static flags = {
    config: Flags.string({
      description: 'configuration name to use',
      default: null,
    }),
    ...BaseCommand.flags,
  }

  async run() {
    const configFile = this.container.resolve('configFile');

    let configName;
    if (this.parsedFlags.config !== null) {
      if (!configFile.isConfigExists(this.parsedFlags.config)) {
        throw new ConfigIsNotPresentError(this.parsedFlags.config);
      }

      configName = this.parsedFlags.config;
    } else {
      const defaultConfigName = configFile.getDefaultConfigName();

      if (defaultConfigName === null) {
        throw new Error(`Default config is not set.

You probably need to set up a node with the 'dashmate setup' command first.

You can also use the '--config' option, or set the default config with 'dashmate config default'`);
      }

      if (!configFile.isConfigExists(defaultConfigName)) {
        throw new Error(`Default config ${defaultConfigName} does not exist. Please use '--config' option or change default config`);
      }

      configName = defaultConfigName;
    }

    const config = configFile.getConfig(configName);

    this.container.register({
      config: asValue(config),
    });

    return super.run();
  }
}
