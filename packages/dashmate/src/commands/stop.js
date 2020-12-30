const { Listr } = require('listr2');

const BaseCommand = require('../oclif/command/BaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

class StopCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
    },
    dockerCompose,
    config,
  ) {
    const tasks = new Listr([
      {
        title: 'Stop node',
        task: async () => dockerCompose.stop(config.toEnvs()),
      },
    ],
    {
      renderer: isVerbose ? 'verbose' : 'default',
      rendererOptions: {
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run();
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

StopCommand.description = `Stop node

Stop node
`;

StopCommand.flags = {
  ...BaseCommand.flags,
};

module.exports = StopCommand;
