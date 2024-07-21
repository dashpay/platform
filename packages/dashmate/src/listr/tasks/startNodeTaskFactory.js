import { Listr } from 'listr2';
import { Observable } from 'rxjs';
import { NETWORK_LOCAL } from '../../constants.js';
import isServiceBuildRequired from '../../util/isServiceBuildRequired.js';

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
export default function startNodeTaskFactory(
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
      // Ensure log files for Drive are created
      const loggers = config.get('platform.drive.abci.logs');
      Object.values(loggers)
        .filter((logger) => logger.destination !== 'stdout' && logger.destination !== 'stderr')
        .forEach((logger) => {
          ensureFileMountExists(logger.destination, 0o666);
        });

      // Ensure access log files for Gateway are created
      config.get('platform.gateway.log.accessLogs')
        .filter((log) => log.type === 'file')
        .forEach((log) => {
          ensureFileMountExists(log.path, 0o666);
        });

      // Ensure tenderdash log file is created
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
            user: 'dashmate',
            pass: config.get('core.rpc.users.dashmate.password'),
            host: await getConnectionHost(config, 'core', 'core.rpc.host'),
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
