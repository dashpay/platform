const wait = require('../util/wait');

/**
 *
 * @typedef waitForBlocksWithSDK
 * @param {DAPIClient} dapiClient
 * @param {number} numberOfBlocks
 * @param {function(confirmations: number)} progressCallback
 * @return {Promise<void>}
 */
async function waitForBlocksWithSDK(dapiClient, numberOfBlocks, progressCallback = () => {}) {
  let { blocks: currentBlock } = await dapiClient.getStatus();

  const lastBlock = currentBlock + numberOfBlocks;
  do {
    ({ blocks: currentBlock } = await dapiClient.getStatus());

    await progressCallback(numberOfBlocks - (lastBlock - currentBlock));

    if (currentBlock < lastBlock) {
      await wait(30000);
    }
  } while (currentBlock < lastBlock);
}

module.exports = waitForBlocksWithSDK;
