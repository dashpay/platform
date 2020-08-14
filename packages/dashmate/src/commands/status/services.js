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

    if (config.options.network !== 'testnet') {
      Object.assign(serviceHumanNames, {
        drive_mongodb_replica_init: 'Initiate Drive MongoDB replica',
        drive_mongodb: 'Drive MongoDB',
        drive_abci: 'Drive ABCI',
        drive_tendermint: 'Drive Tendermint',
        dapi_insight: 'DAPI Insight',
        dapi_api: 'DAPI API',
        dapi_tx_filter_stream: 'DAPI Transactions Filter Stream',
        dapi_envoy: 'DAPI Envoy',
        dapi_nginx: 'DAPI Nginx',
      });
    }

    const tableRows = [
      ['Service', 'Container ID', 'Version', 'Status'],
    ];

    for (const [serviceName, serviceDescription] of Object.entries(serviceHumanNames)) {
      let containerId;
      let status;
      let exitCode;
      let version;

      try {
        ({
          Id: containerId,
          State: {
            Status: status,
            ExitCode: exitCode,
          },
          Config: {
            Labels: {
              'org.dash.version': version,
            },
          },
        } = await dockerCompose.inspectService(config.toEnvs(), serviceName));
      } catch (e) {
        if (e instanceof ContainerIsNotPresentError) {
          status = 'not started';
        }
      }

      if (serviceName === 'drive_mongodb_replica_init' && status === 'exited' && exitCode === 0) {
        // noinspection UnnecessaryContinueJS
        continue;
      }

      let statusText;
      if (status) {
        statusText = chalk.keyword(status === 'running' ? 'green' : 'red')(status);
      }

      tableRows.push([
        serviceDescription,
        containerId ? containerId.slice(0, 12) : undefined,
        version,
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
