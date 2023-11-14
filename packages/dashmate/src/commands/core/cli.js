import { Args } from '@oclif/core';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import ServiceIsNotRunningError from '../../docker/errors/ServiceIsNotRunningError.js';

export default class CliCommand extends ConfigBaseCommand {
  static description = 'Dash Core CLI';

  static args = {
    command: Args.string({
      name: 'command',
      required: true,
      description: 'dash core command written in the double quotes',
    }),
  };

  static flags = {
    ...ConfigBaseCommand.flags,
  };

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
