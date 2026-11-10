import wait from '../../util/wait.js';
import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js';
import getDkgStopBlockers, {
  getBlockersAt,
  hasMembershipActiveDkgs,
} from './getDkgStopBlockers.js';

/**
 * How long a safe result from Core older than 24.1 must hold before it is
 * trusted. Core's DKG worker records a session it is a member of shortly
 * after the session's start block connects; this is comfortably longer.
 */
export const SAFE_STOP_CONFIRMATION_DELAY_MS = 15000;

/**
 * @param {RpcClient} rpcClient
 * @return {Promise<number|null>} Core's numeric `version`, or `null` if
 *   Core does not report one
 */
export async function readCoreVersion(rpcClient) {
  const { result } = await rpcClient.getNetworkInfo();

  return Number.isInteger(result && result.version) ? result.version : null;
}

/**
 * Read Core's DKG state at one chain tip. Each RPC reads the tip on its own,
 * so the tip is read before and after the DKG calls; if a block connected in
 * between, they may describe different tips and are discarded.
 *
 * @param {RpcClient} rpcClient
 * @return {Promise<Object|null>}
 */
async function readDkgSnapshot(rpcClient) {
  const { result: before } = await rpcClient.getBlockchainInfo();
  const { result: dkgInfo } = await rpcClient.quorum('dkginfo');
  const { result: dkgStatus } = await rpcClient.quorum('dkgstatus');
  const { result: after } = await rpcClient.getBlockchainInfo();

  if (before.bestblockhash !== after.bestblockhash) {
    return null;
  }

  return {
    height: after.blocks,
    hash: after.bestblockhash,
    isSyncing: before.initialblockdownload === true && after.initialblockdownload === true,
    dkgInfo,
    dkgStatus,
  };
}

/**
 * Core older than 24.1 reports `active_dkgs` as the number of the DKG
 * worker's session records, which `quorum dkgstatus` lists. Different counts
 * mean the worker changed its records between the two reads, as it does
 * when a session starts.
 *
 * @param {Object} snapshot
 * @param {number|null} coreVersion
 * @return {boolean}
 */
function haveSessionRecordsChanged({ dkgInfo, dkgStatus }, coreVersion) {
  return !hasMembershipActiveDkgs(coreVersion)
    && Number.isInteger(dkgInfo && dkgInfo.active_dkgs)
    && Array.isArray(dkgStatus && dkgStatus.session)
    && dkgInfo.active_dkgs !== dkgStatus.session.length;
}

/**
 * @typedef {Object} SafeStopCheck
 * @property {string} status `safe`, `blocked`, or `unsettled` when Core's
 *   state changed too fast to read a consistent or confirmed one
 * @property {number|null} height
 * @property {string} [hash] chain tip the check read
 * @property {DkgStopBlocker[]} blockers see {@link getDkgStopBlockers}
 * @property {boolean} [isSyncing] Core is in initial block download
 */

/**
 * @param {RpcClient} rpcClient
 * @param {number|null} coreVersion
 * @return {Promise<SafeStopCheck>}
 */
async function evaluate(rpcClient, coreVersion) {
  const snapshot = await readDkgSnapshot(rpcClient);

  if (!snapshot) {
    return { status: 'unsettled', height: null, blockers: [] };
  }

  const { height, hash, isSyncing } = snapshot;

  // Core's DKG worker does nothing during initial block download, and Core
  // never re-enters it without a restart, so a node in it has had no DKG
  // session since it started.
  if (isSyncing) {
    return {
      status: 'safe', height, hash, blockers: [], isSyncing,
    };
  }

  if (haveSessionRecordsChanged(snapshot, coreVersion)) {
    return {
      status: 'unsettled', height, hash, blockers: [],
    };
  }

  const blockers = getDkgStopBlockers({ ...snapshot, coreVersion });

  return {
    status: getBlockersAt(blockers, height).length > 0 ? 'blocked' : 'safe',
    height,
    hash,
    blockers,
  };
}

/**
 * Query Core and decide whether stopping the masternode now would disrupt a
 * DKG it takes part in. See {@link getDkgStopBlockers} for the rule.
 *
 * Core older than 24.1 reports a session only once its DKG worker has
 * recorded it, shortly after the start block connects, so a safe result is
 * checked again after {@link SAFE_STOP_CONFIRMATION_DELAY_MS}. By then every
 * block the first check saw is old enough to have its sessions recorded.
 * The first check already ruled out a DKG of this node starting within
 * `MIN_BLOCKS_BEFORE_DKG` blocks, so the second may see up to that many new
 * blocks; more than that, a lower tip, or a different block at the first
 * check's height is `unsettled`.
 *
 * @param {RpcClient} rpcClient
 * @param {number|null} coreVersion from {@link readCoreVersion}
 * @return {Promise<SafeStopCheck>}
 */
export default async function checkMasternodeSafeToStop(rpcClient, coreVersion) {
  const first = await evaluate(rpcClient, coreVersion);

  if (first.status !== 'safe' || first.isSyncing || hasMembershipActiveDkgs(coreVersion)) {
    return first;
  }

  await wait(SAFE_STOP_CONFIRMATION_DELAY_MS);

  const second = await evaluate(rpcClient, coreVersion);

  const hasChainMoved = second.height < first.height
    || (second.height === first.height && second.hash !== first.hash)
    || second.height > first.height + MIN_BLOCKS_BEFORE_DKG;

  if (second.status === 'safe' && !second.isSyncing && hasChainMoved) {
    return { ...second, status: 'unsettled' };
  }

  return second;
}
