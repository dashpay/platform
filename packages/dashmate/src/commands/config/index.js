const { inspect } = require('util');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class ConfigCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {renderServiceTemplates} renderServiceTemplates
   * @param {writeServiceConfigs} writeServiceConfigs
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    config,
    renderServiceTemplates,
    writeServiceConfigs,
  ) {
    const output = `${config.getName()} config:\n\n${inspect(
      config.getOptions(),
      { colors: true, depth: null, maxArrayLength: 2 },
    )}`;

    const serviceConfigs = renderServiceTemplates(config);
    writeServiceConfigs(config.getName(), serviceConfigs);

    // eslint-disable-next-line no-console
    console.log(output);
  }
}

ConfigCommand.description = 'Show default config';

ConfigCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = ConfigCommand;
