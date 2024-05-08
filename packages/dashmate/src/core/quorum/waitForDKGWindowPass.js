import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js';
import wait from '../../util/wait.js';

/**
 * @param {RpcClient} rpcClient
 * @return {Promise<void>}
 */
export default async function waitForDKGWindowPass(rpcClient) {
  let startBlockCount;
  let startNextDkg;

  let isInDKG = true;

  do {
    const [currentBlockCount, currentDkgInfo] = await Promise
      .all([rpcClient.getBlockCount(), rpcClient.quorum('dkginfo')]);

    const { result: blockCount } = currentBlockCount;
    const { result: dkgInfo } = currentDkgInfo;

    const { next_dkg: nextDkg } = dkgInfo;

    if (!startBlockCount) {
      startBlockCount = blockCount;
    }

    if (!startNextDkg) {
      startNextDkg = nextDkg;
    }

    isInDKG = nextDkg <= MIN_BLOCKS_BEFORE_DKG;

    if (isInDKG && blockCount > startBlockCount + startNextDkg + 1) {
      throw new Error(`waitForDKGWindowPass deadline exceeded: dkg did not happen for ${startBlockCount + nextDkg + 1} ${startNextDkg + 1} blocks`);
    }

    await wait(10000);
  }
  while (isInDKG);
}
