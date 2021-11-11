const { flags: flagTypes } = require('@oclif/command');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class StatusCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {outputStatusOverview} outputStatusOverview
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    outputStatusOverview,
    config,
  ) {
    await outputStatusOverview(config, flags.format);
  }
}

StatusCommand.description = 'Show status overview';

StatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: flagTypes.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = StatusCommand;
