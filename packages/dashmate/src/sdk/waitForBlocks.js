const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const PRESETS = require('../presets');

const wait = require('../util/wait');

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {string} preset
 * @param {string} network
 * @param {number} numberOfBlocks
 * @return {Promise<void>}
 */
async function waitForBlocks(dapiClient, preset, network, numberOfBlocks) {
  if (preset === PRESETS.LOCAL) {
    const privateKey = new PrivateKey();

    await dapiClient.generateToAddress(
      numberOfBlocks,
      privateKey.toAddress(network).toString(),
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
}

module.exports = waitForBlocks;
