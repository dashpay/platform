import { Listr } from 'listr2';
import DashCoreLib from '@dashevo/dashcore-lib';
import { NETWORK_LOCAL } from '../../constants.js';
import isServiceBuildRequired from '../../util/isServiceBuildRequired.js';

const { PrivateKey } = DashCoreLib;

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {waitForCorePeersConnected} waitForCorePeersConnected
 * @param {waitForMasternodesSync} waitForMasternodesSync
 * @param {createRpcClient} createRpcClient
 * @param {Docker} docker
 * @param {startNodeTask} startNodeTask
 * @param {waitForNodeToBeReadyTask} waitForNodeToBeReadyTask
 * @param {buildServicesTask} buildServicesTask
 * @param {getConnectionHost} getConnectionHost
 * @return {startGroupNodesTask}
 */
export default function startGroupNodesTaskFactory(
  dockerCompose,
  waitForCorePeersConnected,
  waitForMasternodesSync,
  createRpcClient,
  docker,
  startNodeTask,
  waitForNodeToBeReadyTask,
  buildServicesTask,
  getConnectionHost,
) {
  /**
   * @typedef {startGroupNodesTask}
   * @param {Config[]} configGroup
   * @return {Object}
   */
  function startGroupNodesTask(configGroup) {
    const minerConfig = configGroup.find((config) => (
      config.get('core.miner.enable')
    ));

    const platformBuildConfig = configGroup.find((config) => (
      isServiceBuildRequired(config)
    ));

    return new Listr([
      {
        enabled: () => platformBuildConfig,
        task: () => buildServicesTask(platformBuildConfig),
      },
      {
        title: 'Starting nodes',
        task: async (ctx) => {
          ctx.skipBuildServices = true;

          const tasks = configGroup.map((config) => ({
            title: `Starting ${config.getName()} node`,
            task: () => startNodeTask(config),
          }));

          return new Listr(tasks, { concurrent: true });
        },
      },
      {
        title: 'Wait for Core peers to be connected',
        enabled: () => minerConfig && minerConfig.get('network') === NETWORK_LOCAL,
        task: () => {
          const tasks = configGroup.map((config) => ({
            title: `Checking ${config.getName()} peers`,
            task: async () => {
              const rpcClient = createRpcClient({
                port: config.get('core.rpc.port'),
                user: 'dashmate',
                pass: config.get('core.rpc.users.dashmate.password'),
                host: await getConnectionHost(config, 'core', 'core.rpc.host'),
              });

              await waitForCorePeersConnected(rpcClient);
            },
          }));

          return new Listr(tasks, { concurrent: true });
        },
      },
      {
        title: 'Start a miner',
        enabled: () => minerConfig && minerConfig.get('network') === NETWORK_LOCAL,
        task: async () => {
          let minerAddress = minerConfig.get('core.miner.address');

          if (minerAddress === null) {
            const privateKey = new PrivateKey();
            minerAddress = privateKey.toAddress('regtest').toString();

            minerConfig.set('core.miner.address', minerAddress);
          }

          const minerInterval = minerConfig.get('core.miner.interval');

          await dockerCompose.execCommand(
            minerConfig,
            'core',
            [
              'bash',
              '-c',
              `while true; do
                dash-cli generatetoaddress 1 ${minerAddress};
                sleep ${minerInterval};
              done`,
            ],
            ['--detach'],
          );
        },
      },
      {
        title: 'Wait for nodes to be ready',
        enabled: (ctx) => Boolean(ctx.waitForReadiness),
        task: () => {
          const tasks = configGroup
            .filter((config) => config.get('platform.enable'))
            .map((config) => ({
              title: `Wait for ${config.getName()} node`,
              task: () => waitForNodeToBeReadyTask(config),
            }));

          return new Listr(tasks, { concurrent: true });
        },
      },
    ]);
  }

  return startGroupNodesTask;
}
