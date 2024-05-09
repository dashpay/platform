const wait = require('./wait');

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {number} numberOfBlocks
 * @return {Promise<void>}
 */
module.exports = async function waitForBlocks(dapiClient, numberOfBlocks) {
  let { chain: { blocksCount: currentBlockHeight } } = await dapiClient.core.getBlockchainStatus();

  const desiredBlockHeight = currentBlockHeight + numberOfBlocks;
  do {
    ({ chain: { blocksCount: currentBlockHeight } } = await dapiClient.core.getBlockchainStatus());

    if (currentBlockHeight < desiredBlockHeight) {
      await wait(5000);
    }
  } while (currentBlockHeight < desiredBlockHeight);
};
