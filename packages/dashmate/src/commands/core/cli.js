const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const ServiceIsNotRunningError = require('../../docker/errors/ServiceIsNotRunningError');

class CliCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   *
   * @param dockerCompose
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    config,
    dockerCompose,
  ) {
    const { command } = args;

    if (!(await dockerCompose.isServiceRunning(config, 'core'))) {
      throw new ServiceIsNotRunningError(config.name, 'core');
    }

    const { out } = await dockerCompose.execCommand(config, 'core', `dash-cli ${command}`);

    // eslint-disable-next-line no-console
    console.log(out.trim());

    return out;
  }
}

CliCommand.description = 'Dash Core CLI';

CliCommand.args = [{
  name: 'command',
  required: true,
  description: 'dash core command written in the double quotes',
}];

CliCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = CliCommand;
