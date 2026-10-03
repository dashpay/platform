import wait from '../../util/wait.js';
import isMasternodeSafeToStopDuringDkg, {
  needsSafeStopConfirmation,
  shouldInspectDkgStatusForSafeStop,
} from './isMasternodeSafeToStopDuringDkg.js';

// Comfortably longer than Core's delay between connecting a block and
// tracking a DKG session that starts at it.
export const SAFE_STOP_CONFIRMATION_DELAY_MS = 5000;

/**
 * @param {RpcClient} rpcClient
 * @return {Promise<{ isSafe: boolean, dkgInfo: Object }>}
 */
async function evaluateSafeToStop(rpcClient) {
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

  return {
    isSafe: isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, currentHeight),
    dkgInfo,
  };
}

/**
 * Query Core and decide whether the masternode can be stopped without
 * disrupting a DKG session. See {@link isMasternodeSafeToStopDuringDkg}
 * for the rule and {@link needsSafeStopConfirmation} for why a safe
 * verdict is sometimes re-checked.
 *
 * @param {RpcClient} rpcClient
 * @return {Promise<boolean>}
 */
export default async function checkMasternodeSafeToStop(rpcClient) {
  const { isSafe, dkgInfo } = await evaluateSafeToStop(rpcClient);

  if (!isSafe || !needsSafeStopConfirmation(dkgInfo)) {
    return isSafe;
  }

  await wait(SAFE_STOP_CONFIRMATION_DELAY_MS);

  return (await evaluateSafeToStop(rpcClient)).isSafe;
}
