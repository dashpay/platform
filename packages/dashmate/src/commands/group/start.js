const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

class GroupStartCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {startNodeTask} startNodeTask
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      update: isUpdate,
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
      verbose: isVerbose,
    },
    dockerCompose,
    startNodeTask,
    configGroup,
  ) {
    const groupName = configGroup[0].get('group');

    const tasks = new Listr(
      [
        {
          title: `Start ${groupName} nodes`,
          task: async () => (
            new Listr(configGroup.map((config) => (
              {
                title: `Starting ${config.getName()} node`,
                task: () => startNodeTask(
                  config,
                  {
                    driveImageBuildPath,
                    dapiImageBuildPath,
                    isUpdate,
                  },
                ),
              }
            )))
          ),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
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

GroupStartCommand.description = 'Start group nodes';

GroupStartCommand.flags = {
  ...GroupBaseCommand.flags,
  update: flagTypes.boolean({ char: 'u', description: 'download updated services before start', default: false }),
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
};

module.exports = GroupStartCommand;
