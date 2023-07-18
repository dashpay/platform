const { Flags } = require('@oclif/core');
const chalk = require('chalk');

const ConfigBaseCommand = require('../oclif/command/ConfigBaseCommand');

const { OUTPUT_FORMATS } = require('../constants');
const printArrayOfObjects = require('../printers/printArrayOfObjects');

class UpdateCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {string} format
   * @param {docker} docker
   * @param {Config} config
   * @param updateNode
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      format,
    },
    docker,
    config,
    updateNode,
  ) {
    const updateInfo = await updateNode(config);

    // Draw table or show json
    printArrayOfObjects(updateInfo
      .reduce((acc, {
        name, title, updated, image,
      }) => ([
        ...acc,
        format === OUTPUT_FORMATS.PLAIN
          ? { Service: title, Image: image, Updated: updated ? chalk.yellow('updated') : chalk.green('up to date') }
          : {
            name, title, updated, image,
          },
      ]),
      []), format);
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
