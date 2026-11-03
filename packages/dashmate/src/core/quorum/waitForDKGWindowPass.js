import wait from '../../util/wait.js';
import checkMasternodeSafeToStop from './checkMasternodeSafeToStop.js';

const CHECK_INTERVAL_MS = 10000;

/**
 * Poll Core until the masternode is safe to stop without disrupting a
 * DKG session. See {@link checkMasternodeSafeToStop} for the
 * safety rule. The only acceptable exit is reaching a safe state, so
 * that `--safe` cannot silently fall back to an unsafe restart.
 *
 * @param {RpcClient} rpcClient
 * @return {Promise<void>}
 */
export default async function waitForDKGWindowPass(rpcClient) {
  while (!await checkMasternodeSafeToStop(rpcClient)) {
    await wait(CHECK_INTERVAL_MS);
  }
}
