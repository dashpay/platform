const logger = require('../../../../logger');

module.exports = async function getBestBlockHeader() {
  logger.silly('DAPIClientWrapper.getBestBlockHeader');
  return this.getBlockHeaderByHash(await this.getBestBlockHash());
};
