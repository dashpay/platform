import wait from '../../util/wait.js';
import isMasternodeSafeToStopDuringDkg, {
  shouldInspectDkgStatusForSafeStop,
} from './isMasternodeSafeToStopDuringDkg.js';

const CHECK_INTERVAL_MS = 10000;

/**
 * Poll Core until the masternode is safe to stop without disrupting a
 * DKG session. See {@link isMasternodeSafeToStopDuringDkg} for the
 * safety rule. The only acceptable exit is reaching a safe state, so
 * that `--safe` cannot silently fall back to an unsafe restart.
 *
 * @param {RpcClient} rpcClient
 * @return {Promise<void>}
 */
export default async function waitForDKGWindowPass(rpcClient) {
  for (;;) {
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

    if (isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, currentHeight)) {
      return;
    }

    await wait(CHECK_INTERVAL_MS);
  }
}
