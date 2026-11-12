/* eslint-disable no-console */
import { Listr } from 'listr2';
import waitForDKGWindowPass from '../../core/quorum/waitForDKGWindowPass.js';
import checkMasternodeSafeToStop, {
  readCoreVersion,
} from '../../core/quorum/checkMasternodeSafeToStop.js';
import formatDkgStopStatus, {
  formatDkgStopRefusal,
} from '../../core/quorum/formatDkgStopStatus.js';
import {
  NETWORK_DEVNET,
  NETWORK_LOCAL,
  NETWORK_MAINNET,
  NETWORK_TESTNET,
} from '../../constants.js';

/**
 * Group commands stop a local network's miner before its masternodes. Without
 * it no block comes, so a DKG wait could never end, and with every masternode
 * of the network going down together their DKG produces no commitment to
 * penalize them in. Only masternodes of that local network, the miner's group,
 * are exempt: a group may hold configs of other networks.
 *
 * @param {Config} config
 * @param {Object} ctx
 * @return {boolean}
 */
function isLocalNetworkHalted(config, ctx) {
  return config.get('network') === NETWORK_LOCAL
    && !!ctx.stoppedLocalMinerGroups
    && ctx.stoppedLocalMinerGroups.has(config.get('group'));
}

// A check comes back unsettled while the chain moves too fast to read a
// consistent DKG state, such as while Core catches up after a restart.
const SAFE_STOP_CHECK_ATTEMPTS = 3;

// Dash's target block spacing, which difficulty adjustment holds mainnet and
// testnet to on average.
const TARGET_BLOCK_SECONDS = 150;

/**
 * Block spacing to estimate waits with, or `null` where blocks come on their
 * own schedule, as on local networks.
 *
 * @param {Config} config
 * @return {number|null}
 */
function getTargetBlockSeconds(config) {
  switch (config.get('network')) {
    case NETWORK_MAINNET:
    case NETWORK_TESTNET:
      return TARGET_BLOCK_SECONDS;
    case NETWORK_DEVNET:
      return config.get('core.devnet.powTargetSpacing');
    default:
      return null;
  }
}

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
          && !ctx.platformOnly && !isLocalNetworkHalted(config, ctx),
        task: async (ctx) => {
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

          if (check.status !== 'safe') {
            // Commands pass the flags they have, so `undefined` means the
            // running command offers no such flag.
            throw new Error(formatDkgStopRefusal(check, {
              blockTimeSeconds: getTargetBlockSeconds(config),
              canWait: ctx.isSafe !== undefined,
              canForce: ctx.isForce !== undefined,
            }));
          }
        },
      },
      {
        title: 'Wait for DKG window to pass',
        enabled: (ctx) => config.get('core.masternode.enable') && !ctx.isForce && ctx.isSafe
          && !ctx.platformOnly && !isLocalNetworkHalted(config, ctx),
        task: async (ctx, task) => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: 'dashmate',
            pass: config.get('core.rpc.users.dashmate.password'),
            host: await getConnectionHost(config, 'core', 'core.rpc.host'),
          });

          const blockTimeSeconds = getTargetBlockSeconds(config);
          let output;

          try {
            await waitForDKGWindowPass(rpcClient, {
              // Without a terminal, every output change is printed as a new line,
              // so the output changes only when there is something new to say.
              progressCallback: (status) => {
                const text = formatDkgStopStatus(status, { blockTimeSeconds });

                if (text !== output) {
                  output = text;
                  // eslint-disable-next-line no-param-reassign
                  task.output = text;
                }
              },
            });
          } catch (e) {
            if (!e.blocker) {
              throw e;
            }

            throw new Error(formatDkgStopRefusal(
              { status: 'blocked', height: null, blockers: [e.blocker] },
              { blockTimeSeconds, canWait: false, canForce: ctx.isForce !== undefined },
            ), { cause: e });
          }
        },
      },
      {
        title: `Stopping ${config.getName()} node`,
        task: async (ctx) => {
          const profiles = ctx.platformOnly
            ? selectPlatformProfiles(config, { includeAll: true })
            : [];

          await dockerCompose.stop(config, { profiles });

          if (config.get('network') === NETWORK_LOCAL && config.get('core.miner.enable')) {
            ctx.stoppedLocalMinerGroups = new Set([
              ...(ctx.stoppedLocalMinerGroups || []),
              config.get('group'),
            ]);
          }
        },
      },
    ]);
  }

  return stopNodeTask;
}
