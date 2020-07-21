const { is } = require('../../../utils');

/**
 * Get a getBlockHeader from a provided block hash or block height
 * @param {string|number} identifier - Block Hash or blockHeight
 * @return {Promise<BlockHeader>}
 */
async function getBlockHeader(identifier) {
  const search = await this.storage.searchBlockHeader(identifier);
  if (search.found) {
    return search.result;
  }
  const blockHeight = (is.num(identifier)) ? identifier : null;
  const blockHeader = (is.num(identifier))
    ? await this.transport.getBlockByHeight(blockHeight)
    : await this.transport.getBlockHeaderByHash(identifier);

  if (this.cacheBlockHeaders) {
    await this.storage.importBlockHeader(blockHeader, blockHeight);
  }
  return blockHeader;
}
module.exports = getBlockHeader;
