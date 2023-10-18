const { Flags } = require('@oclif/core');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class DashCliCommand extends ConfigBaseCommand {
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

    const { out } = await dockerCompose.execCommand(config, 'core', `dash-cli ${command}`);

    // eslint-disable-next-line no-console
    console.log(out.trim());

    return out;
  }
}

DashCliCommand.description = 'Dash Core CLI`';

DashCliCommand.args = [{
  name: 'command',
  required: true,
  description: 'Execute a dash-cli command on the core container of the given node config',
}];

DashCliCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = DashCliCommand;
