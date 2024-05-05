import { MIN_BLOCKS_BEFORE_DKG } from '../../constants.js'
import wait from '../../util/wait.js'

/**
 * @param {RpcClient} rpcClient
 * @return {Promise<void>}
 */
export default async function waitForDKGWindowPass (rpcClient) {
  const { result: dkgInfo } = await rpcClient.quorum('dkginfo')
  const { result: blockchainInfo } = await rpcClient.getBlockchainInfo()

  const { active_dkgs: activeDkgs, next_dkg: nextDkg } = dkgInfo
  const { blocks: startBlock } = blockchainInfo

  let isInDKG = true

  while (isInDKG) {
    await wait(1000)

    const { result: blockchainInfo } = await rpcClient.getBlockchainInfo()

    isInDKG = activeDkgs !== 0 || nextDkg < MIN_BLOCKS_BEFORE_DKG

    if (blockchainInfo.blocks > startBlock + nextDkg) {
      throw new Error(`waitForDKGWindowPass deadline exceeded: dkg did not happen for ${startBlock + nextDkg} blocks`);
    }
  }
}
