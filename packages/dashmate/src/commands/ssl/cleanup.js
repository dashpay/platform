import { Listr } from 'listr2';
import { Flags } from '@oclif/core';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../../oclif/errors/MuteOneLineError.js';

export default class CleanupCommand extends ConfigBaseCommand {
  static description = `Cleanup Zero SSL certificate

Cancel all drafted or pending validation certificates on ZeroSSL
`;

  static flags = {
    ...ConfigBaseCommand.flags,
    verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {boolean} flags.verbose
   * @param {Config} config
   * @param {cleanupZeroSSLCertificatesTask} cleanupZeroSSLCertificatesTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    config,
    cleanupZeroSSLCertificatesTask,
  ) {
    const tasks = new Listr(
      [
        {
          title: 'Cleanup ZeroSSL certificate',
          task: async () => cleanupZeroSSLCertificatesTask(config),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          showTimer: isVerbose,
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
          removeEmptyLines: false,
        },
      },
    );

    try {
      await tasks.run();
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
