const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

/**
 *
 * @typedef generateBlocksWithSDK
 * @param {DAPIClient} dapiClient
 * @param {string} network
 * @param {number} numberOfBlocks
 * @param {function(balance: number)} progressCallback
 * @return {Promise<void>}
 */
async function generateBlocksWithSDK(
  dapiClient,
  network,
  numberOfBlocks,
  progressCallback = () => {},
) {
  const privateKey = new PrivateKey();
  const address = privateKey.toAddress(network).toString();

  let generatedBlocks = 0;

  do {
    const blockHashes = await dapiClient.generateToAddress(
      1,
      address,
    );

    generatedBlocks += blockHashes.length;

    if (blockHashes.length > 0) {
      await progressCallback(generatedBlocks);
    }
  } while (generatedBlocks < numberOfBlocks);
}

module.exports = generateBlocksWithSDK;
