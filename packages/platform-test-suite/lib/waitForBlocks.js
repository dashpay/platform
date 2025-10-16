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
  let attempts = 0;

  do {
    currentBlockHeight = await dapiClient.core.getBestBlockHeight();

    if (currentBlockHeight < desiredBlockHeight) {
      attempts += 1;
      await wait(
        5000,
        `best block height ${desiredBlockHeight} (current ${currentBlockHeight}, attempt ${attempts})`,
      );
    }
  } while (currentBlockHeight < desiredBlockHeight);
};
