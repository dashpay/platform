import { Listr } from 'listr2';

import { Flags } from '@oclif/core';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';

export default class StartCommand extends ConfigBaseCommand {
  static description = 'Start node';

  static flags = {
    ...ConfigBaseCommand.flags,
    'wait-for-readiness': Flags.boolean({ char: 'w', description: 'wait for nodes to be ready', default: false }),
    platform: Flags.boolean({ char: 'p', description: 'start only platform', default: false }),
    force: Flags.boolean({
      char: 'f',
      description: 'force start even if any services are already running',
      default: false,
    }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {startNodeTask} startNodeTask
   * @param {waitForNodeToBeReadyTask} waitForNodeToBeReadyTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      'wait-for-readiness': waitForReadiness,
      verbose: isVerbose,
      force: isForce,
      platform: platformOnly,
    },
    dockerCompose,
    startNodeTask,
    waitForNodeToBeReadyTask,
    config,
  ) {
    const tasks = new Listr(
      [
        {
          title: `Start ${config.getName()} node`,
          task: () => startNodeTask(config),
        },
        {
          title: 'Wait for nodes to be ready',
          enabled: () => waitForReadiness,
          task: () => waitForNodeToBeReadyTask(config),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          showTimer: isVerbose,
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
        },
      },
    );

    try {
      await tasks.run({
        isVerbose,
        isForce,
        platformOnly: platformOnly === true,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
