const { table } = require('table');
const chalk = require('chalk');

const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');

const BaseCommand = require('../../oclif/command/BaseCommand');

const PRESETS = require('../../presets');

class ServicesStatusCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset,
    },
    flags,
    dockerCompose,
  ) {
    const serviceHumanNames = {
      core: 'Core',
      sentinel: 'Sentinel',
    };

    if (preset !== 'testnet') {
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
        } = await dockerCompose.inspectService(preset, serviceName));
      } catch (e) {
        if (e instanceof ContainerIsNotPresentError) {
          status = 'not started';
        }
      }

      if (serviceName === 'drive_mongodb_replica_init' && status === 'exited' && exitCode === 0) {
        // noinspection UnnecessaryContinueJS
        continue;
      }

      tableRows.push([
        serviceDescription,
        containerId.slice(0, 12),
        version,
        chalk.keyword(status === 'running' ? 'green' : 'red')(status),
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

ServicesStatusCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: Object.values(PRESETS),
}];

module.exports = ServicesStatusCommand;
