import { Listr }  from 'listr2';

import { Flags } from '@oclif/core';
import {GroupBaseCommand} from "../../oclif/command/GroupBaseCommand.js";
import {MuteOneLineError} from "../../oclif/errors/MuteOneLineError.js";

export class GroupStartCommand extends GroupBaseCommand {

  static flags = {
    ...GroupBaseCommand.flags,
    'wait-for-readiness': Flags.boolean({ char: 'w', description: 'wait for nodes to be ready', default: false }),
  };

  static description = 'Start group nodes';
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {startNodeTask} startNodeTask
   * @param {Config[]} configGroup
   * @param {startGroupNodesTask} startGroupNodesTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      'wait-for-readiness': waitForReadiness,
      verbose: isVerbose,
    },
    dockerCompose,
    startNodeTask,
    configGroup,
    startGroupNodesTask,
  ) {
    const groupName = configGroup[0].get('group');

    const tasks = new Listr(
      [
        {
          title: `Start ${groupName} nodes`,
          task: () => startGroupNodesTask(configGroup),
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
        waitForReadiness,
        isVerbose,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
