import wait from '../../util/wait.js';
import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js';
import getDkgStopBlockers, {
  getAnnouncedNonMemberDkgs,
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
 * Which stop rule applies depends on Core's version, so a version Core does
 * not report is an error rather than a guess.
 *
 * @param {RpcClient} rpcClient
 * @return {Promise<number>} Core's numeric `version`
 */
export async function readCoreVersion(rpcClient) {
  const { result } = await rpcClient.getNetworkInfo();

  if (!Number.isInteger(result && result.version)) {
    throw new Error('Core did not report its version');
  }

  return result.version;
}

/**
 * Read Core's DKG state at one chain tip. Each RPC reads the tip on its own,
 * so the tip is read before and after the DKG calls; if a block connected in
 * between, they may describe different tips and are discarded. Core in
 * initial block download takes no part in DKGs, so that state holds however
 * the tip moves.
 *
 * With `ancestorHeight`, also reads the hash of the active chain's block at
 * that height, at the same tip.
 *
 * @param {RpcClient} rpcClient
 * @param {number} [ancestorHeight]
 * @return {Promise<Object|null>}
 */
async function readDkgSnapshot(rpcClient, ancestorHeight) {
  const { result: before } = await rpcClient.getBlockchainInfo();

  let ancestorHash;
  if (ancestorHeight !== undefined && before.blocks >= ancestorHeight) {
    ({ result: ancestorHash } = await rpcClient.getBlockHash(ancestorHeight));
  }

  const { result: dkgInfo } = await rpcClient.quorum('dkginfo');
  const { result: dkgStatus } = await rpcClient.quorum('dkgstatus');
  const { result: after } = await rpcClient.getBlockchainInfo();

  const isSyncing = before.initialblockdownload === true && after.initialblockdownload === true;

  if (!isSyncing && before.bestblockhash !== after.bestblockhash) {
    return null;
  }

  return {
    height: after.blocks,
    hash: after.bestblockhash,
    ancestorHash,
    isSyncing,
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
 * @param {number} coreVersion
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
 * @property {Set<string>} nonMemberDkgs DKGs Core announced without this
 *   node, see {@link getAnnouncedNonMemberDkgs}
 * @property {boolean} [isSyncing] Core is in initial block download
 * @property {boolean} [isReorg] the chain no longer has the tip of an earlier
 *   check; what was learned on that branch, such as `nonMemberDkgs`, no
 *   longer holds
 * @property {{ height: number, hash: string }|null} tip chain tip
 *   `nonMemberDkgs` were read on, or `null` when no consistent read was made
 */

/**
 * @param {RpcClient} rpcClient
 * @param {number} coreVersion
 * @param {Set<string>} knownNonMemberDkgs
 * @param {number} [ancestorHeight]
 * @return {Promise<SafeStopCheck>}
 */
async function evaluate(rpcClient, coreVersion, knownNonMemberDkgs, ancestorHeight) {
  const snapshot = await readDkgSnapshot(rpcClient, ancestorHeight);

  if (!snapshot) {
    return {
      status: 'unsettled', height: null, blockers: [], nonMemberDkgs: new Set(), tip: null,
    };
  }

  const { height, hash, isSyncing } = snapshot;
  const result = {
    height,
    hash,
    ancestorHash: snapshot.ancestorHash,
    nonMemberDkgs: getAnnouncedNonMemberDkgs(snapshot.dkgInfo, height),
    tip: { height, hash },
  };

  // Core's DKG worker does nothing during initial block download, and Core
  // never re-enters it without a restart, so a node in it has had no DKG
  // session since it started.
  if (isSyncing) {
    return {
      ...result, status: 'safe', blockers: [], isSyncing,
    };
  }

  if (haveSessionRecordsChanged(snapshot, coreVersion)) {
    return { ...result, status: 'unsettled', blockers: [] };
  }

  const blockers = getDkgStopBlockers({ ...snapshot, coreVersion, knownNonMemberDkgs });

  return {
    ...result,
    status: getBlockersAt(blockers, height).length > 0 ? 'blocked' : 'safe',
    blockers,
  };
}

/**
 * Query Core and decide whether stopping the masternode now would disrupt a
 * DKG it takes part in. See {@link getDkgStopBlockers} for the rule.
 *
 * Core older than 24.1 reports a session only once its DKG worker has
 * recorded it, after the start block connects, so a safe result is checked
 * again after {@link SAFE_STOP_CONFIRMATION_DELAY_MS}. By then every block the
 * first check saw is old enough for Core to have replaced records of previous
 * rounds. The first check already ruled out a DKG of this node starting
 * within `MIN_BLOCKS_BEFORE_DKG` blocks, so the second may see up to that
 * many new blocks on top of the first check's tip; more than that, a lower
 * tip, or a chain without the first check's tip is `unsettled`.
 *
 * `knownNonMemberDkgs` may only hold what earlier checks learned on the
 * current chain: pass the tip of the latest of them as `ancestor`, and a
 * check that finds it replaced reports `isReorg`.
 *
 * @param {RpcClient} rpcClient
 * @param {number} coreVersion from {@link readCoreVersion}
 * @param {Object} [options]
 * @param {Set<string>} [options.knownNonMemberDkgs] DKGs Core announced
 *   without this node at earlier checks
 * @param {{ height: number, hash: string }} [options.ancestor] tip of the
 *   latest earlier check
 * @return {Promise<SafeStopCheck>}
 */
export default async function checkMasternodeSafeToStop(
  rpcClient,
  coreVersion,
  { knownNonMemberDkgs = new Set(), ancestor } = {},
) {
  const first = await evaluate(rpcClient, coreVersion, knownNonMemberDkgs, ancestor && ancestor.height);

  if (ancestor && first.height !== null
    && (first.height < ancestor.height || first.ancestorHash !== ancestor.hash)) {
    return { ...first, status: 'unsettled', isReorg: true };
  }

  if (first.status !== 'safe' || first.isSyncing || hasMembershipActiveDkgs(coreVersion)) {
    return first;
  }

  await wait(SAFE_STOP_CONFIRMATION_DELAY_MS);

  const nonMemberDkgs = new Set([...knownNonMemberDkgs, ...first.nonMemberDkgs]);
  const second = await evaluate(rpcClient, coreVersion, nonMemberDkgs, first.height);

  // The confirming read was discarded: keep what the first check verified.
  if (second.height === null) {
    return { ...second, nonMemberDkgs, tip: first.tip };
  }

  // The first check's tip was replaced: none of it holds on this chain.
  if (second.height < first.height || second.ancestorHash !== first.hash) {
    return { ...second, status: 'unsettled', isReorg: true };
  }

  second.nonMemberDkgs = new Set([...nonMemberDkgs, ...second.nonMemberDkgs]);

  if (second.status === 'safe' && !second.isSyncing
    && second.height > first.height + MIN_BLOCKS_BEFORE_DKG) {
    return { ...second, status: 'unsettled' };
  }

  return second;
}
