import isMasternodeSafeToStopDuringDkg, {
  shouldInspectDkgStatusForSafeStop,
} from './isMasternodeSafeToStopDuringDkg.js';

/**
 * Query Core and decide whether stopping would disrupt a DKG session.
 * Older Core responses require the existing per-session status checks.
 *
 * @param {RpcClient} rpcClient
 * @return {Promise<boolean>}
 */
export default async function checkMasternodeSafeToStop(rpcClient) {
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

  return isMasternodeSafeToStopDuringDkg(dkgInfo, dkgStatus, currentHeight);
}
