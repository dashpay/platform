import wait from '../../util/wait.js';
import checkMasternodeSafeToStop, { readCoreVersion } from './checkMasternodeSafeToStop.js';
import { getBlockersAt } from './getDkgStopBlockers.js';

export const CHECK_INTERVAL_MS = 10000;

/**
 * Poll Core until the masternode is safe to stop without disrupting a
 * DKG session. See {@link checkMasternodeSafeToStop} for the
 * safety rule. The only acceptable exit is reaching a safe state, so
 * that `--safe` cannot silently fall back to an unsafe restart.
 *
 * DKGs this node takes part in are remembered from the polls that saw them
 * announced. Once a session starts Core no longer announces it, and Core
 * older than 24.1 reports it only after its DKG worker has computed the
 * session's members and recorded it, which can take a while at the start
 * of a rotation cycle.
 *
 * @param {RpcClient} rpcClient
 * @return {Promise<void>}
 */
export default async function waitForDKGWindowPass(rpcClient) {
  const coreVersion = await readCoreVersion(rpcClient);

  const announcedDkgs = new Map();

  for (;;) {
    const check = await checkMasternodeSafeToStop(rpcClient, coreVersion);

    if (check.height !== null) {
      check.blockers
        .filter(({ reason, until }) => reason === 'upcoming' && until !== null)
        .forEach((dkg) => {
          announcedDkgs.set(`${dkg.llmqType}:${dkg.quorumIndex}:${dkg.from}`, dkg);
        });

      announcedDkgs.forEach((dkg, key) => {
        if (dkg.until <= check.height) {
          announcedDkgs.delete(key);
        }
      });
    }

    if (check.status === 'safe'
      && (check.isSyncing || getBlockersAt([...announcedDkgs.values()], check.height).length === 0)) {
      return;
    }

    await wait(CHECK_INTERVAL_MS);
  }
}
