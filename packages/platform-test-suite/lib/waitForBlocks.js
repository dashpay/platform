const wait = require('./wait');

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {number} numberOfBlocks
 * @return {Promise<void>}
 */
module.exports = async function waitForBlocks(dapiClient, numberOfBlocks) {
  let { chain: { blocksCount: currentBlockHeight } } = await dapiClient.core.getStatus();

  const desiredBlockHeight = currentBlockHeight + numberOfBlocks;
  do {
    ({ chain: { blocksCount: currentBlockHeight } } = await dapiClient.core.getStatus());

    if (currentBlockHeight < desiredBlockHeight) {
      await wait(5000);
    }
  } while (currentBlockHeight < desiredBlockHeight);
};
