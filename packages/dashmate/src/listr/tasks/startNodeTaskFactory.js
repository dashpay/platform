const { Listr } = require('listr2');
const { Observable } = require('rxjs');

const { NETWORK_LOCAL } = require('../../constants');
const isServiceBuildRequired = require('../../util/isServiceBuildRequired');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {waitForCorePeersConnected} waitForCorePeersConnected
 * @param {waitForMasternodesSync} waitForMasternodesSync
 * @param {createRpcClient} createRpcClient
 * @param {buildServicesTask} buildServicesTask
 * @param {getConnectionHost} getConnectionHost
 * @param {ensureFileMountExists} ensureFileMountExists
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
   * @return {Object}
   */
  function startNodeTask(config) {
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
      ensureFileMountExists(prettyFilePath, 0o666);

      const jsonFilePath = config.get('platform.drive.abci.log.jsonFile.path');
      ensureFileMountExists(jsonFilePath, 0o666);

      const tenderdashLogFilePath = config.get('platform.drive.tenderdash.log.path');
      if (tenderdashLogFilePath !== null) {
        ensureFileMountExists(tenderdashLogFilePath, 0o666);
      }
    }

    return new Listr([
      {
        title: 'Check node is not started',
        enabled: (ctx) => !ctx.isForce,
        task: async (ctx) => {
          const profiles = [];
          if (ctx.platformOnly) {
            profiles.push('platform');
          }

          if (await dockerCompose.isNodeRunning(config, { profiles })) {
            throw new Error('Running services detected. Please ensure all services are stopped for this config before starting');
          }
        },
      },
      {
        title: 'Check core is started',
        enabled: (ctx) => ctx.platformOnly === true,
        task: async () => {
          if (!await dockerCompose.isServiceRunning(config, 'core')) {
            throw new Error('Platform services depend on Core and can\'t be started without it. Please run "dashmate start" without "--platform" flag');
          }
        },
      },
      {
        enabled: (ctx) => !ctx.skipBuildServices
          && isServiceBuildRequired(config),
        task: () => buildServicesTask(config),
      },
      {
        title: 'Start services',
        task: async (ctx) => {
          const isMasternode = config.get('core.masternode.enable');
          if (isMasternode) {
            // Check operatorPrivateKey is set
            config.get('core.masternode.operator.privateKey', true);
          }

          const profiles = [];
          if (ctx.platformOnly) {
            profiles.push('platform');
          }

          await dockerCompose.up(config, { profiles });
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
