import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js';
import wait from '../../util/wait.js';

/**
 * @param {RpcClient} rpcClient
 * @return {Promise<void>}
 */
export default async function waitForDKGWindowPass(rpcClient) {
  const { result: startBlockchainInfo } = await rpcClient.getBlockchainInfo();
  const { blocks: startBlock } = startBlockchainInfo;

  const { result: startNextDKGInfo } = await rpcClient.quorum('dkginfo');
  const { next_dkg: startNextDKG } = startNextDKGInfo;

  let isInDKG = true;

  while (isInDKG) {
    await wait(1000);

    const { result: dkgInfo } = await rpcClient.quorum('dkginfo');
    const { next_dkg: nextDkg } = dkgInfo;

    const { result: blockchainInfo } = await rpcClient.getBlockchainInfo();

    isInDKG = nextDkg <= MIN_BLOCKS_BEFORE_DKG;

    console.log('1', isInDKG, nextDkg, blockchainInfo.blocks, startBlock, startNextDKG)

    if (isInDKG && blockchainInfo.blocks > startBlock + startNextDKG + 1) {
      throw new Error(`waitForDKGWindowPass deadline exceeded: dkg did not happen for ${startBlock + nextDkg + 1} ${startNextDKG + 1} blocks`);
    }
  }
}
