const { Listr } = require('listr2');

const BaseCommand = require('../oclif/command/BaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

class ResetCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    dockerCompose,
    config,
  ) {
    const tasks = new Listr([
      {
        title: 'Reset node data',
        task: () => (
          new Listr([
            {
              title: 'Remove Docker containers and associated data',
              task: async () => dockerCompose.down(config.toEnvs()),
            },
          ])
        ),
      },
    ],
    {
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

ResetCommand.description = `Reset node data

Reset node data
`;

ResetCommand.flags = {
  ...BaseCommand.flags,
};

module.exports = ResetCommand;
