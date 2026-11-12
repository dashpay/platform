import wait from '../../util/wait.js';
import checkMasternodeSafeToStop, { readCoreVersion } from './checkMasternodeSafeToStop.js';
import { getBlockersAt, isDkgKeyCurrent } from './getDkgStopBlockers.js';

export const CHECK_INTERVAL_MS = 10000;

// Unreadable DKG data does not clear while dashmate and Core stay as they
// are; a few polls rule out a read that caught Core mid-update.
export const MAX_UNREADABLE_POLLS = 3;

// About a minute of failing RPC calls, enough to ride out Core being busy
// or briefly unreachable without ending a long wait.
export const MAX_FAILED_POLLS = 6;

/**
 * Poll Core until the masternode is safe to stop without disrupting a
 * DKG session. See {@link checkMasternodeSafeToStop} for the
 * safety rule. It resolves only once that is safe, so that `--safe` cannot
 * silently fall back to an unsafe restart, and throws, leaving the node
 * running, when Core's DKG data stays unreadable or Core keeps failing to
 * answer.
 *
 * DKGs this node takes part in, announced or running, are remembered until
 * their Commit phase ends. Once a session starts Core no longer announces
 * it, Core older than 24.1 reports it only after its DKG worker has computed
 * the session's members and recorded it, which can take a while at the
 * start of a rotation cycle, and a restarted Core keeps no record of it at
 * all. DKGs announced without this node are remembered too, so their
 * sessions need not wait for a record that never comes, as long as the
 * chain they were announced on stays: a reorg replacing it may make this
 * node a member after all.
 *
 * `progressCallback` receives one of:
 * - `{ type: 'blocked', height, blockers }`: blockers at and above `height`,
 *   remembered DKGs included
 * - `{ type: 'confirming', height }`: a safe result waits to be confirmed
 * - `{ type: 'unsettled', height }`: Core's state changed while reading it
 * - `{ type: 'rpc-error', error, failedPolls, maxFailedPolls }`
 *
 * The error for unreadable data carries the `blocker` that could not be
 * read.
 *
 * @param {RpcClient} rpcClient
 * @param {Object} [options]
 * @param {function(Object)} [options.progressCallback]
 * @return {Promise<void>}
 */
export default async function waitForDKGWindowPass(rpcClient, { progressCallback = () => {} } = {}) {
  const knownDkgs = new Map();
  let nonMemberDkgs = new Set();
  let lastTip;

  let failedPolls = 0;
  let unreadablePolls = 0;

  // Take in what a check learned: the DKGs it saw announced, with or without
  // this node, and the tip it read them on.
  const remember = (check) => {
    nonMemberDkgs = check.isReorg
      ? check.nonMemberDkgs
      : new Set([...nonMemberDkgs, ...check.nonMemberDkgs]);

    if (check.tip) {
      lastTip = check.tip;
    }

    if (check.height === null) {
      return;
    }

    nonMemberDkgs = new Set([...nonMemberDkgs]
      .filter((key) => isDkgKeyCurrent(key, check.height)));

    check.blockers
      .filter(({ from, until }) => from !== null && until !== null)
      .forEach((dkg) => {
        knownDkgs.set(`${dkg.reason}:${dkg.llmqType}:${dkg.quorumIndex}:${dkg.from}`, dkg);
      });

    knownDkgs.forEach((dkg, key) => {
      if (dkg.until <= check.height) {
        knownDkgs.delete(key);
      }
    });
  };

  for (;;) {
    let check;

    try {
      // Which rule applies depends on the version, and Core may have been
      // replaced since the last poll.
      const coreVersion = await readCoreVersion(rpcClient);

      check = await checkMasternodeSafeToStop(rpcClient, coreVersion, {
        knownNonMemberDkgs: nonMemberDkgs,
        ancestor: lastTip,
        beforeConfirmation: (first) => {
          // Keep what the first check learned even if its confirmation is cut
          // short, as by Core restarting and forgetting its announcements.
          remember(first);

          // A remembered DKG blocks this height whatever the confirmation finds.
          if (getBlockersAt([...knownDkgs.values()], first.height).length > 0) {
            return false;
          }

          progressCallback({ type: 'confirming', height: first.height });

          return true;
        },
      });

      failedPolls = 0;
    } catch (e) {
      failedPolls += 1;

      if (failedPolls >= MAX_FAILED_POLLS) {
        throw new Error(`Core did not answer ${MAX_FAILED_POLLS} checks in a row, `
          + `so the node was not stopped:\n${e.message}`, { cause: e });
      }

      progressCallback({
        type: 'rpc-error', error: e, failedPolls, maxFailedPolls: MAX_FAILED_POLLS,
      });
    }

    if (check) {
      remember(check);
    }

    // Unreadable data counts only over polls that read it in a row.
    if (!check) {
      unreadablePolls = 0;
    } else if (check.status === 'unsettled') {
      unreadablePolls = 0;
      progressCallback({ type: 'unsettled', height: check.height });
    } else {
      const blockers = [
        // Bounded blockers are among the remembered ones.
        ...check.blockers.filter(({ from, until }) => from === null || until === null),
        ...knownDkgs.values(),
      ];
      const current = getBlockersAt(blockers, check.height);

      // A remembered DKG blocks even while Core syncs: Core restarted into
      // initial block download may finish in time for it.
      if (check.status === 'safe' && current.length === 0) {
        return;
      }

      const unreadable = current.find(({ reason }) => reason === 'unreadable');
      unreadablePolls = unreadable ? unreadablePolls + 1 : 0;

      if (unreadablePolls >= MAX_UNREADABLE_POLLS) {
        const error = new Error(`dashmate cannot read Core's DKG data: ${unreadable.source}`);
        error.blocker = unreadable;

        throw error;
      }

      progressCallback({ type: 'blocked', height: check.height, blockers });
    }

    await wait(CHECK_INTERVAL_MS);
  }
}
