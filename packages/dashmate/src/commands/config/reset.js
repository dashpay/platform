const BaseCommand = require('../../oclif/command/BaseCommand');

class ConfigResetCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {resetSystemConfig} resetSystemConfig
   * @param {Config} config
   * @param {ConfigCollection} configCollection
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    resetSystemConfig,
    config,
    configCollection,
  ) {
    resetSystemConfig(configCollection, config.getName());

    // eslint-disable-next-line no-console
    console.log(`${config.getName()} is reset to factory settings`);
  }
}

ConfigResetCommand.description = `Reset config

Reset system configuration to factory settings
`;

ConfigResetCommand.flags = {
  ...BaseCommand.flags,
};

module.exports = ConfigResetCommand;
