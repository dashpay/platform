const { Listr } = require('listr2');
const { Flags } = require('@oclif/core');
const chalk = require('chalk');

const ConfigBaseCommand = require('../oclif/command/ConfigBaseCommand');

const printObject = require('../printers/printObject');
const { OUTPUT_FORMATS } = require('../constants');
const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

class UpdateCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {string} format
   * @param {boolean} isVerbose
   * @param {docker} docker
   * @param {Config} config
   * @param updateNodeTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      format,
      verbose: isVerbose,
    },
    docker,
    config,
    updateNodeTask,
  ) {
    const tasks = new Listr(
      [
        {
          title: `Update ${config.getName()} node`,
          task: () => updateNodeTask(config),
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
      const updated = await tasks.run({
        isVerbose,
        format,
      });

      // Draw table or show json
      printObject(updated
        .reduce((acc, {
          serviceName, title, pulled, image,
        }) => ([
          ...acc,
          format === OUTPUT_FORMATS.PLAIN
            ? [title, image, pulled ? chalk.yellow('updated') : chalk.green('up to date')]
            : {
              serviceName, title, pulled, image,
            },
        ]),
        []), format, false);
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

UpdateCommand.description = 'Update node software';

UpdateCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = UpdateCommand;
