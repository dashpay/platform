const chalk = require('chalk');

const { flags: flagTypes } = require('@oclif/command');
const { OUTPUT_FORMATS } = require('../../constants');

const printArrayOfObjects = require('../../printers/printArrayOfObjects');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');

const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');

class ServicesStatusCommand extends ConfigBaseCommand {
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
    const serviceHumanNames = {
      core: 'Core',
    };

    if (config.get('core.masternode.enable')) {
      Object.assign(serviceHumanNames, {
        sentinel: 'Sentinel',
      });
    }

    if (config.get('network') !== 'mainnet') {
      Object.assign(serviceHumanNames, {
        drive_mongodb: 'Drive MongoDB',
        drive_abci: 'Drive ABCI',
        drive_tenderdash: 'Drive Tenderdash',
        dapi_api: 'DAPI API',
        dapi_tx_filter_stream: 'DAPI Transactions Filter Stream',
        dapi_envoy: 'DAPI Envoy',
      });
    }

    const outputRows = [];

    for (const [serviceName, serviceDescription] of Object.entries(serviceHumanNames)) {
      let containerId;
      let status;
      let image;

      try {
        ({
          Id: containerId,
          State: {
            Status: status,
          },
          Config: {
            Image: image,
          },
        } = await dockerCompose.inspectService(config.toEnvs(), serviceName));
      } catch (e) {
        if (e instanceof ContainerIsNotPresentError) {
          status = 'not started';
        }
      }

      let statusText;
      if (status) {
        statusText = (status === 'running' ? chalk.green : chalk.red)(status);
      }

      outputRows.push({
        Service: serviceDescription,
        'Container ID': containerId ? containerId.slice(0, 12) : undefined,
        Image: image,
        Status: statusText,
      });
    }

    printArrayOfObjects(outputRows, flags.format);
  }
}

ServicesStatusCommand.description = 'Show service status details';

ServicesStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: flagTypes.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = ServicesStatusCommand;
