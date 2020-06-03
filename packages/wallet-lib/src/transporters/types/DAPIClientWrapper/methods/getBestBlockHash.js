const logger = require('../../../../logger');

module.exports = async function getBestBlockHash() {
  logger.silly('DAPIClientWrapper.getBestBlockHash');
  return this.client.getBestBlockHash();
};
