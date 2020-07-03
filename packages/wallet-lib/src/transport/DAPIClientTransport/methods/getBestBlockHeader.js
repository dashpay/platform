const logger = require('../../../logger');

module.exports = async function getBestBlockHeader() {
  logger.silly('DAPIClientTransport.getBestBlockHeader');

  return this.getBlockHeaderByHash(await this.getBestBlockHash());
};
