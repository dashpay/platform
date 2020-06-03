const logger = require('../../../../logger');

module.exports = async function getBestBlock() {
  logger.silly('DAPIClientWrapper.getBestBlock');
  return this.getBlockByHash(await this.getBestBlockHash());
};
