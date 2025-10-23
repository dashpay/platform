/* eslint-disable no-console */
import { Listr } from 'listr2';
import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js';
import waitForDKGWindowPass from '../../core/quorum/waitForDKGWindowPass.js';

/**
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @param {getConfigProfiles} getConfigProfiles
 * @return {stopNodeTask}
 */
export default function stopNodeTaskFactory(
  dockerCompose,
  createRpcClient,
  getConnectionHost,
  getConfigProfiles,
) {
  function getPlatformProfiles(config) {
    const platformProfiles = getConfigProfiles(config)
      .filter((profile) => profile.startsWith('platform'));

    if (platformProfiles.length === 0) {
      platformProfiles.push('platform');
    }

    return Array.from(new Set(platformProfiles));
  }

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
          const profiles = ctx.platformOnly ? getPlatformProfiles(config) : [];

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
        title: `Stopping ${config.getName()} node`,
        task: async (ctx) => {
          const profiles = ctx.platformOnly ? getPlatformProfiles(config) : [];

          await dockerCompose.stop(config, { profiles });
        },
      },
    ]);
  }

  return stopNodeTask;
}
