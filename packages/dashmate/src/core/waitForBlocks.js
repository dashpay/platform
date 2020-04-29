const wait = require('../util/wait');

/**
 * Wait for blocks to be generated
 * @typedef waitForBlocks
 * @param {CoreService} coreService
 * @param {number} blocks
 * @param {function(confirmations: number)} progressCallback
 * @returns {Promise<void>}
 */
async function waitForBlocks(coreService, blocks, progressCallback = () => {}) {
  let { result: currentBlock } = await coreService.getRpcClient().getBlockCount();
  const lastBlock = currentBlock + blocks;

  do {
    await wait(20000);

    ({ result: currentBlock } = await coreService.getRpcClient().getBlockCount());

    await progressCallback(blocks - (lastBlock - currentBlock));
  } while (currentBlock < lastBlock);
}


module.exports = waitForBlocks;
