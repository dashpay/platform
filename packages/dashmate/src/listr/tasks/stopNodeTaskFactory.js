/* eslint-disable no-console */
import { Listr } from 'listr2';
import waitForDKGWindowPass from '../../core/quorum/waitForDKGWindowPass.js';
import checkMasternodeSafeToStop, {
  readCoreVersion,
} from '../../core/quorum/checkMasternodeSafeToStop.js';
import { getBlockersAt } from '../../core/quorum/getDkgStopBlockers.js';
import { NETWORK_LOCAL } from '../../constants.js';

// A check comes back unsettled while the chain moves too fast to read a
// consistent DKG state, such as while Core catches up after a restart.
const SAFE_STOP_CHECK_ATTEMPTS = 3;

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
        enabled: (ctx) => config.get('core.masternode.enable') && !ctx.isForce && !ctx.isSafe
          && !ctx.platformOnly && !ctx.isLocalMinerStopped,
        task: async () => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: 'dashmate',
            pass: config.get('core.rpc.users.dashmate.password'),
            host: await getConnectionHost(config, 'core', 'core.rpc.host'),
          });

          const coreVersion = await readCoreVersion(rpcClient);

          let check;
          for (let attempt = 0; attempt < SAFE_STOP_CHECK_ATTEMPTS; attempt += 1) {
            check = await checkMasternodeSafeToStop(rpcClient, coreVersion);

            if (check.status !== 'unsettled') {
              break;
            }
          }

          const unreadable = check.status === 'blocked'
            && getBlockersAt(check.blockers, check.height)
              .find(({ reason }) => reason === 'unreadable');

          if (unreadable) {
            throw new Error(`dashmate cannot read Core's DKG state (${unreadable.source}). `
              + 'Update dashmate, or continue with --force');
          }

          if (check.status === 'blocked') {
            throw new Error('Your node is currently participating in a DKG exchange session '
              + '(or one is about to start) and stopping it right now may result in a PoSe ban. '
              + 'Try again later, or continue with --force or --safe flags');
          }

          if (check.status === 'unsettled') {
            throw new Error('The chain kept advancing while checking for DKG sessions '
              + '(is Core still syncing?). '
              + 'Try again later, or continue with --force or --safe flags');
          }
        },
      },
      {
        title: 'Wait for DKG window to pass',
        enabled: (ctx) => config.get('core.masternode.enable') && !ctx.isForce && ctx.isSafe
          && !ctx.platformOnly && !ctx.isLocalMinerStopped,
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

          // Group commands stop a local network's miner before its masternodes.
          // Without it no block comes, so a DKG wait could never end, and with
          // every masternode of the network going down together their DKG
          // produces no commitment to penalize them in.
          if (config.get('network') === NETWORK_LOCAL && config.get('core.miner.enable')) {
            ctx.isLocalMinerStopped = true;
          }
        },
      },
    ]);
  }

  return stopNodeTask;
}
