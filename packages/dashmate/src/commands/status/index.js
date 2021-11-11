const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const getFormat = require('../../util/getFormat');

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
    await outputStatusOverview(config, getFormat(flags));
  }
}

StatusCommand.description = 'Show status overview';

StatusCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = StatusCommand;
