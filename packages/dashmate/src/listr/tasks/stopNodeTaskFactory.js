/* eslint-disable no-console */
import { Listr } from 'listr2';
import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js';
import waitForDKGWindowPass from '../../core/quorum/waitForDKGWindowPass.js';

/**
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @return {stopNodeTask}
 */
export default function stopNodeTaskFactory(
  dockerCompose,
  createRpcClient,
  getConnectionHost,
) {
  /**
   * Stop node
   * @typedef stopNodeTask
   * @param {Config} config
   *
   * @return {Listr}
   */
  function stopNodeTask(config) {
    return new Listr([
      {
        title: 'Check node is running',
        skip: (ctx) => ctx.isForce,
        task: async (ctx) => {
          const profiles = [];
          if (ctx.platformOnly) {
            profiles.push('platform');
          }

          if (!await dockerCompose.isNodeRunning(config, { profiles })) {
            throw new Error('Node is not running');
          }
        },
      },
      {
        title: 'Check node is participating in DKG',
        enabled: (ctx) => config.get('core.masternode.enable') && !ctx.isForce && !ctx.isSafe && !ctx.platformOnly,
        task: async () => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: 'dashmate',
            pass: config.get('core.rpc.users.dashmate.password'),
            host: await getConnectionHost(config, 'core', 'core.rpc.host'),
          });

          const { result: dkgInfo } = await rpcClient.quorum('dkginfo');
          const { next_dkg: nextDkg } = dkgInfo;

          if (nextDkg <= MIN_BLOCKS_BEFORE_DKG) {
            throw new Error('Your node is currently participating in DKG exchange session and '
              + 'stopping it right now may result in PoSE ban. Try again later, or continue with --force or --safe flags');
          }
        },
      },
      {
        title: 'Wait for DKG window to pass',
        enabled: (ctx) => config.get('core.masternode.enable') && !ctx.isForce && ctx.isSafe && !ctx.platformOnly,
        task: async () => waitForDKGWindowPass(createRpcClient({
          port: config.get('core.rpc.port'),
          user: 'dashmate',
          pass: config.get('core.rpc.users.dashmate.password'),
          host: await getConnectionHost(config, 'core', 'core.rpc.host'),
        })),
      },
      {
        title: 'Save core node time',
        enabled: () => config.get('group') === 'local',
        skip: (ctx) => ctx.isForce,
        task: async () => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: 'dashmate',
            pass: config.get('core.rpc.users.dashmate.password'),
            host: await getConnectionHost(config, 'core', 'core.rpc.host'),
          });

          const { result: { mediantime } } = await rpcClient.getBlockchainInfo();

          config.set('core.miner.mediantime', mediantime);
        },
      },
      {
        title: `Stopping ${config.getName()} node`,
        task: async (ctx) => {
          const profiles = [];
          if (ctx.platformOnly) {
            profiles.push('platform');
          }
          await dockerCompose.stop(config, { profiles });
        },
      },
    ]);
  }

  return stopNodeTask;
}
