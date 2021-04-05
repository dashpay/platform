const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const ConfigBaseCommand = require('../oclif/command/ConfigBaseCommand');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

class ResetCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {isSystemConfig} isSystemConfig
   * @param {Config} config
   * @param {resetNodeTask} resetNodeTask
   *
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      hard: isHardReset,
      'platform-only': isPlatformOnlyReset,
    },
    isSystemConfig,
    config,
    resetNodeTask,
  ) {
    if (isHardReset && !isSystemConfig(config.getName())) {
      throw new Error(`Cannot hard reset non-system config "${config.getName()}"`);
    }

    if (!config.has('platform') && isPlatformOnlyReset) {
      throw new Error('Cannot reset platform only if platform services are not enabled in config');
    }

    const tasks = new Listr([
      {
        title: `Reset ${config.getName()} node`,
        task: () => resetNodeTask(config),
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
    });

    try {
      await tasks.run({
        isHardReset,
        isPlatformOnlyReset,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

ResetCommand.description = `Reset node data

Reset node data
`;

ResetCommand.flags = {
  ...ConfigBaseCommand.flags,
  hard: flagTypes.boolean({ char: 'h', description: 'reset config as well as data', default: false }),
  'platform-only': flagTypes.boolean({ char: 'p', description: 'reset platform data only', default: false }),
};

module.exports = ResetCommand;
