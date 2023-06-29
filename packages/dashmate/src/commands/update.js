const { Flags } = require('@oclif/core');
const chalk = require('chalk');

const ConfigBaseCommand = require('../oclif/command/ConfigBaseCommand');

const printObject = require('../printers/printObject');
const { OUTPUT_FORMATS } = require('../constants');

class UpdateCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {string} format
   * @param {docker} docker
   * @param {Config} config
   * @param updateNodeTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      format,
    },
    docker,
    config,
    updateNodeTask,
  ) {
    const updated = await updateNodeTask(config);

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
