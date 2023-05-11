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
 * @param getConnectionHost {getConnectionHost}
 * @param ensureFileMountExists {ensureFileMountExists}
 * @return {startNodeTask}
 */
function startNodeTaskFactory(
  dockerCompose,
  waitForCorePeersConnected,
  waitForMasternodesSync,
  createRpcClient,
  buildServicesTask,
  getConnectionHost,
  ensureFileMountExists,
) {
  /**
   * @typedef {startNodeTask}
   * @param {Config} config
   * @param {Object} [options={}]
   * @param {boolean} [options.platformOnly=false]
   * @return {Object}
   */
  function startNodeTask(config, options = {}) {
    // check core is not reindexing
    if (config.get('core.reindex.enable', true)) {
      throw new Error(`Your dashcore node in config [${config.name}] is reindexing, please allow the process to complete first`);
    }

    // Check external IP is set
    if (config.get('core.masternode.enable')) {
      config.get('externalIp', true);
    }

    const isMinerEnabled = config.get('core.miner.enable');

    if (isMinerEnabled === true && config.get('network') !== NETWORK_LOCAL) {
      throw new Error(`'core.miner.enable' option only works with local network. Your network is ${config.get('network')}.`);
    }

    const coreLogFilePath = config.get('core.log.file.path');
    ensureFileMountExists(coreLogFilePath, 0o666);

    // Check Drive log files are created
    if (config.get('platform.enable')) {
      const prettyFilePath = config.get('platform.drive.abci.log.prettyFile.path');
      ensureFileMountExists(prettyFilePath);

      const jsonFilePath = config.get('platform.drive.abci.log.jsonFile.path');
      ensureFileMountExists(jsonFilePath);
    }

    return new Listr([
      {
        title: 'Check node is not started',
        task: async () => {
          if (await dockerCompose.isServiceRunning(config.toEnvs(options))) {
            throw new Error('Running services detected. Please ensure all services are stopped for this config before starting');
          }
        },
      },
      {
        title: 'Check core is started',
        enabled: options.platformOnly,
        task: async () => {
          if (!await dockerCompose.isServiceRunning(config.toEnvs(), 'core')) {
            throw new Error('Core service is not running. Please ensure core service is running before starting');
          }
        },
      },
      {
        enabled: (ctx) => !ctx.skipBuildServices
          && config.get('platform.enable')
          && config.get('platform.sourcePath') !== null,
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

          const envs = config.toEnvs(options);

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
            host: await getConnectionHost(config, 'core'),
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
