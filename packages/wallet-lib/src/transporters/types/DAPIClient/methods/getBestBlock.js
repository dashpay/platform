const logger = require('../../../../logger');

module.exports = async function getBestBlock() {
  logger.silly('DAPIClient.getBestBlock');
  return this.getBlockByHash(await this.getBestBlockHash());
};
