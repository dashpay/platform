const { is } = require('../../../utils');
const logger = require('../../../logger');

/**
 * Get a getBlockHeader from a provided block hash or block height
 * @param identifier - Block Hash or blockHeight
 * @return {Promise<*>}
 */
async function getBlockHeader(identifier) {
  const search = await this.storage.searchBlockHeader(identifier);
  if (search.found) {
    return search.result;
  }
  const blockHeight = (is.num(identifier)) ? identifier : null;
  const blockHeader = (is.num(identifier))
    ? await this.transporter.getBlockByHeight(blockHeight)
    : await this.transporter.getBlockHeaderByHash(identifier);

  if (this.cacheBlockHeaders) {
    try {
      await this.storage.importBlockHeader(blockHeader, blockHeight);
    } catch (e) {
      logger.error('getBlockHeader', e);
    }
  }
  return blockHeader;
}
module.exports = getBlockHeader;
