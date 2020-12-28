const { table } = require('table');
const chalk = require('chalk');

const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');

const BaseCommand = require('../../oclif/command/BaseCommand');

class ServicesStatusCommand extends BaseCommand {
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
      sentinel: 'Sentinel',
    };

    if (config.options.network !== 'mainnet') {
      Object.assign(serviceHumanNames, {
        drive_mongodb: 'Drive MongoDB',
        drive_abci: 'Drive ABCI',
        drive_tenderdash: 'Drive Tenderdash',
        dapi_insight: 'DAPI Insight',
        dapi_api: 'DAPI API',
        dapi_tx_filter_stream: 'DAPI Transactions Filter Stream',
        dapi_envoy: 'DAPI Envoy',
        dapi_nginx: 'DAPI Nginx',
      });
    }

    const tableRows = [
      ['Service', 'Container ID', 'Image', 'Status'],
    ];

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

      tableRows.push([
        serviceDescription,
        containerId ? containerId.slice(0, 12) : undefined,
        image,
        statusText,
      ]);
    }

    const tableConfig = {
      // singleLine: true,
      drawHorizontalLine: (index, size) => index === 0 || index === 1 || index === size,
    };

    const tableOutput = table(tableRows, tableConfig);

    // eslint-disable-next-line no-console
    console.log(tableOutput);
  }
}

ServicesStatusCommand.description = 'Show service status details';

ServicesStatusCommand.flags = {
  ...BaseCommand.flags,
};

module.exports = ServicesStatusCommand;
