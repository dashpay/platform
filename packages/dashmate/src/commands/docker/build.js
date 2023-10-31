const { Listr } = require('listr2');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');
const isServiceBuildRequired = require('../../util/isServiceBuildRequired');

class BuildCommand extends ConfigBaseCommand {
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
      throw new Error('Non of the services are configured to be built from sources');
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

BuildCommand.description = `Build docker images
Build docker images for services which configured to be built from source
`;

BuildCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = BuildCommand;
