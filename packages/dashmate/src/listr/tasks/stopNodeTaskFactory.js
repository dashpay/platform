/* eslint-disable no-console */
import { Listr } from 'listr2';
import waitForDKGWindowPass from '../../core/quorum/waitForDKGWindowPass.js';
import isMasternodeSafeToStopDuringDkg, {
  shouldInspectDkgStatusForSafeStop,
} from '../../core/quorum/isMasternodeSafeToStopDuringDkg.js';

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
  function selectPlatformProfiles(config, options) {
    return getConfigProfiles(config, options)
      .filter((profile) => profile.startsWith('platform'));
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
          const profiles = ctx.platformOnly
            ? selectPlatformProfiles(config, { includeAll: true })
            : [];

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

          let dkgStatus;
          let currentHeight;
          if (shouldInspectDkgStatusForSafeStop(dkgInfo)) {
            [
              { result: dkgStatus },
              { result: currentHeight },
            ] = await Promise.all([
              rpcClient.quorum('dkgstatus'),
              rpcClient.getBlockCount(),
            ]);
          }

          if (!isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, currentHeight)) {
            throw new Error('Your node is currently participating in a DKG exchange session '
              + '(or one is about to start) and stopping it right now may result in a PoSe ban. '
              + 'Try again later, or continue with --force or --safe flags');
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
          const profiles = ctx.platformOnly
            ? selectPlatformProfiles(config, { includeAll: true })
            : [];

          await dockerCompose.stop(config, { profiles });
        },
      },
    ]);
  }

  return stopNodeTask;
}
