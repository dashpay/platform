const fs = require('fs');
const path = require('path');

const { Listr } = require('listr2');
const { Observable } = require('rxjs');

const { NETWORK_LOCAL } = require('../../constants');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {waitForCorePeersConnected} waitForCorePeersConnected
 * @param {waitForMasternodesSync} waitForMasternodesSync
 * @param {createRpcClient} createRpcClient
 * @param {buildServicesTask} buildServicesTask
 * @return {startNodeTask}
 */
function startNodeTaskFactory(
  dockerCompose,
  waitForCorePeersConnected,
  waitForMasternodesSync,
  createRpcClient,
  buildServicesTask,
) {
  /**
   * @typedef {startNodeTask}
   * @param {Config} config
   * @return {Object}
   */
  function startNodeTask(config) {
    // Check external IP is set
    config.get('externalIp', true);

    const isMinerEnabled = config.get('core.miner.enable');

    if (isMinerEnabled === true && config.get('network') !== NETWORK_LOCAL) {
      throw new Error(`'core.miner.enabled' option only works with local network. Your network is ${config.get('network')}.`);
    }

    // Check Drive log files are created
    if (config.has('platform')) {
      const prettyFilePath = config.get('platform.drive.abci.log.prettyFile.path');

      if (!fs.existsSync(prettyFilePath)) {
        fs.mkdirSync(path.dirname(prettyFilePath), { recursive: true });
        fs.writeFileSync(prettyFilePath, '');
      }

      const jsonFilePath = config.get('platform.drive.abci.log.jsonFile.path');

      if (!fs.existsSync(jsonFilePath)) {
        fs.mkdirSync(path.dirname(jsonFilePath), { recursive: true });
        fs.writeFileSync(jsonFilePath, '');
      }
    }

    return new Listr([
      {
        title: 'Check node is not started',
        task: async () => {
          if (await dockerCompose.isServiceRunning(config.toEnvs())) {
            throw new Error('Running services detected. Please ensure all services are stopped for this config before starting');
          }
        },
      },
      {
        enabled: (ctx) => !ctx.skipBuildServices && config.has('platform')
          && (
            config.get('platform.dapi.api.docker.build.path') !== null
            || config.get('platform.drive.abci.docker.build.path') !== null
          ),
        task: () => buildServicesTask(config),
      },
      {
        title: 'Start services',
        task: async () => {
          const isMasternode = config.get('core.masternode.enable');
          if (isMasternode) {
            // Check operatorPrivateKey is set
            config.get('core.masternode.operator.privateKey', true);
          }

          const envs = config.toEnvs();

          await dockerCompose.up(envs);
        },
      },
      {
        title: 'Force nodes to sync',
        enabled: () => config.get('network') === NETWORK_LOCAL,
        task: async () => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: config.get('core.rpc.user'),
            pass: config.get('core.rpc.password'),
          });

          return new Observable(async (observer) => {
            await waitForMasternodesSync(
              rpcClient,
              (verificationProgress) => {
                observer.next(`${(verificationProgress * 100).toFixed(2)}% complete`);
              },
            );

            observer.complete();

            return this;
          });
        },
      },
    ]);
  }

  return startNodeTask;
}

module.exports = startNodeTaskFactory;
