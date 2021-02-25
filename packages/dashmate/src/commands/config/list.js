const { table } = require('table');

const BaseCommand = require('../../oclif/command/BaseCommand');

class ConfigListCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    configFile,
  ) {
    const rows = configFile.getAllConfigs()
      .map((config) => [config.getName(), config.get('description')]);

    const output = table(rows);

    // eslint-disable-next-line no-console
    console.log(output);
  }
}

ConfigListCommand.description = 'List available configs';

module.exports = ConfigListCommand;
