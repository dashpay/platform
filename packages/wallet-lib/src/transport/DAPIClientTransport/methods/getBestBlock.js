const logger = require('../../../logger');

module.exports = async function getBestBlock() {
  logger.silly('DAPIClientTransport.getBestBlock');

  return this.getBlockByHash(await this.getBestBlockHash());
};
