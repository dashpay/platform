import { Flags } from '@oclif/core';
import chalk from 'chalk';
import { OUTPUT_FORMATS } from '../constants.js';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import printArrayOfObjects from '../printers/printArrayOfObjects.js';

export default class UpdateCommand extends ConfigBaseCommand {
  static description = 'Update node software';

  static flags = {
    ...ConfigBaseCommand.flags,
    format: Flags.string({
      description: 'display output format',
      default: OUTPUT_FORMATS.PLAIN,
      options: Object.values(OUTPUT_FORMATS),
    }),
  };

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

    const colors = {
      updated: chalk.yellow,
      'up to date': chalk.green,
      error: chalk.red,
    };

    // Draw table or show json
    printArrayOfObjects(updateInfo
      .reduce(
        (acc, {
          name, title, updated, image,
        }) => ([
          ...acc,
          format === OUTPUT_FORMATS.PLAIN
            ? { Service: title, Image: image, Updated: colors[updated](updated) }
            : {
              name, title, updated, image,
            },
        ]),
        [],
      ), format);
  }
}
