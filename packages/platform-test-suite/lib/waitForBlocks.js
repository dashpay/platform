const wait = require('./wait');

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {number} numberOfBlocks
 * @return {Promise<void>}
 */
module.exports = async function waitForBlocks(dapiClient, numberOfBlocks) {
  let currentBlockHeight = await dapiClient.core.getBestBlockHeight();

  const desiredBlockHeight = currentBlockHeight + numberOfBlocks;
  do {
    currentBlockHeight = await dapiClient.core.getBestBlockHeight();

    if (currentBlockHeight < desiredBlockHeight) {
      await wait(5000);
    }
  } while (currentBlockHeight < desiredBlockHeight);
};
