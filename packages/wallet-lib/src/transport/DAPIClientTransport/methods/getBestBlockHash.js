const logger = require('../../../logger');

module.exports = async function getBestBlockHash() {
  logger.silly('DAPIClientTransport.getBestBlockHash');

  return this.client.core.getBestBlockHash();
};
