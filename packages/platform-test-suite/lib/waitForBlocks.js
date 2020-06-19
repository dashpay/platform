const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const wait = require('./wait');

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {number} numberOfBlocks
 * @return {Promise<void>}
 */
module.exports = async function waitForBlocks(dapiClient, numberOfBlocks) {
  if (process.env.NETWORK === 'regtest') {
    const privateKey = new PrivateKey();

    await dapiClient.generateToAddress(
      numberOfBlocks,
      privateKey.toAddress(process.env.NETWORK).toString(),
    );
  } else {
    let { blocks: currentBlockHeight } = await dapiClient.getStatus();

    const desiredBlockHeight = currentBlockHeight + numberOfBlocks;
    do {
      ({ blocks: currentBlockHeight } = await dapiClient.getStatus());

      if (currentBlockHeight < desiredBlockHeight) {
        await wait(30000);
      }
    } while (currentBlockHeight < desiredBlockHeight);
  }
};
