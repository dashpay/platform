import { Listr } from 'listr2';
import DashCoreLib from '@dashevo/dashcore-lib';
import { NETWORK_LOCAL } from '../../constants.js';
import isServiceBuildRequired from '../../util/isServiceBuildRequired.js';

const { PrivateKey } = DashCoreLib;
const WAIT_FOR_NODES_TIMEOUT = 60 * 5 * 1000;

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {waitForCorePeersConnected} waitForCorePeersConnected
 * @param {waitForNodesToHaveTheSameHeight} waitForNodesToHaveTheSameHeight
 * @param {createRpcClient} createRpcClient
 * @param {Docker} docker
 * @param {startNodeTask} startNodeTask
 * @param {waitForNodeToBeReadyTask} waitForNodeToBeReadyTask
 * @param {buildServicesTask} buildServicesTask
 * @param {getConnectionHost} getConnectionHost
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {writeConfigTemplates} writeConfigTemplates
 * @return {startGroupNodesTask}
 */
export default function startGroupNodesTaskFactory(
  dockerCompose,
  waitForCorePeersConnected,
  waitForNodesToHaveTheSameHeight,
  createRpcClient,
  docker,
  startNodeTask,
  waitForNodeToBeReadyTask,
  buildServicesTask,
  getConnectionHost,
  configFileRepository,
  writeConfigTemplates,
) {
  /**
   * @typedef {startGroupNodesTask}
   * @param {Config[]} configGroup
   * @return {Object}
   */
  function startGroupNodesTask(configGroup) {
    let coreRpcClients = [];

    const minerConfig = configGroup.find((config) => (
      config.get('core.miner.enable')
    ));
    const isLocalMinerEnabled = () => (
      minerConfig && minerConfig.get('network') === NETWORK_LOCAL
    );

    const platformBuildConfig = configGroup.find((config) => (
      isServiceBuildRequired(config)
    ));

    return new Listr([
      {
        // A caller that has already built the images - restart builds them
        // before it stops anything - says so, and the build is not repeated
        enabled: (ctx) => Boolean(platformBuildConfig) && !ctx.skipBuildServices,
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
        enabled: isLocalMinerEnabled,
        task: async () => {
          coreRpcClients = await Promise.all(configGroup.map(async (config) => (
            createRpcClient({
              port: config.get('core.rpc.port'),
              user: 'dashmate',
              pass: config.get('core.rpc.users.dashmate.password'),
              host: await getConnectionHost(config, 'core', 'core.rpc.host'),
            })
          )));

          const tasks = configGroup.map((config, index) => ({
            title: `Checking ${config.getName()} peers`,
            task: () => waitForCorePeersConnected(coreRpcClients[index]),
          }));

          return new Listr(tasks, { concurrent: true });
        },
      },
      {
        title: 'Wait for Core nodes to have the same height',
        enabled: isLocalMinerEnabled,
        task: () => waitForNodesToHaveTheSameHeight(
          coreRpcClients,
          WAIT_FOR_NODES_TIMEOUT,
        ),
      },
      {
        title: 'Start a miner',
        enabled: isLocalMinerEnabled,
        task: async () => {
          let minerAddress = minerConfig.get('core.miner.address');

          if (minerAddress === null) {
            configFileRepository.update((configFile) => {
              const freshMinerConfig = configFile.getConfig(minerConfig.getName());

              minerAddress = freshMinerConfig.get('core.miner.address');

              if (minerAddress === null) {
                const privateKey = new PrivateKey();
                minerAddress = privateKey.toAddress('regtest').toString();

                freshMinerConfig.set('core.miner.address', minerAddress);
              }
            }, {
              onSaved: (configFile) => writeConfigTemplates(
                configFile.getConfig(minerConfig.getName()),
              ),
            });

            // The running task keeps the object loaded at command startup, so
            // copy the value selected from fresh state back into it.
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
