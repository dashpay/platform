/**
 *
 * @typedef generateBlocks
 * @param {CoreService} coreService
 * @param {number} blocks
 * @param {function(balance: number)} progressCallback
 * @returns {Promise<void>}
 */
async function generateBlocks(
  coreService,
  blocks,
  progressCallback = () => {},
) {
  let generatedBlocks = 0;

  do {
    const { result: blockHashes } = await coreService.getRpcClient().generate(1, 10000000);

    generatedBlocks += blockHashes.length;

    if (blockHashes.length > 0) {
      await progressCallback(generatedBlocks);
    }
  } while (generatedBlocks < blocks);
}


module.exports = generateBlocks;
