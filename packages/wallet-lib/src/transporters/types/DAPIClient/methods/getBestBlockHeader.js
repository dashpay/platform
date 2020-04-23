const logger = require('../../../../logger');

module.exports = async function getBestBlockHeader() {
  logger.silly('DAPIClient.getBestBlockHeader');
  return this.getBlockHeaderByHash(await this.getBestBlockHash());
};
