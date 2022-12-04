const chalk = require('chalk');

const {Flags} = require('@oclif/core');
const {OUTPUT_FORMATS} = require('../../constants');

const printArrayOfObjects = require('../../printers/printArrayOfObjects');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

class ServicesStatusCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {Config} config
   * @param statusProvider statusProvider
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    dockerCompose,
    config,
    statusProvider
  ) {
    const scope = await statusProvider.getServicesScope()

    const outputRows = [];

    for (const [serviceName, serviceDescription] of Object.entries(scope)) {
      const {humanName, containerId, image, status} = serviceDescription
      if (flags.format === OUTPUT_FORMATS.PLAIN) {
        let statusText;
        if (status) {
          statusText = (status === 'running' ? chalk.green : chalk.red)(status);
        }

        outputRows.push({
          Service: humanName,
          'Container ID': containerId ? containerId.slice(0, 12) : undefined,
          Image: image,
          Status: statusText,
        });
      } else {
        outputRows.push({
          service: serviceName,
          containerId,
          image,
          status
        });
      }
    }

    printArrayOfObjects(outputRows, flags.format);
  }
}

ServicesStatusCommand.description = 'Show service status details';

ServicesStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = ServicesStatusCommand;
