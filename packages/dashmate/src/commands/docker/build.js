import { Listr } from 'listr2';
import { ConfigBaseCommand } from '../../oclif/command/ConfigBaseCommand.js';
import { isServiceBuildRequired } from '../../util/isServiceBuildRequired.js';
import { MuteOneLineError } from '../../oclif/errors/MuteOneLineError.js';

export class BuildCommand extends ConfigBaseCommand {
  static description = `Build docker images
Build docker images for services configured to be built from source
`;

  static flags = {
    ...ConfigBaseCommand.flags,
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {buildServicesTask} buildServicesTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    buildServicesTask,
    config,
  ) {
    if (!isServiceBuildRequired(config)) {
      throw new Error('No services are configured to be built from sources');
    }

    const tasks = new Listr(
      [
        {
          task: () => buildServicesTask(config),
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
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}
