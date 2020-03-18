const logger = require('../../../../logger');

module.exports = async function getBestBlockHash() {
  logger.silly('DAPIClient.getBestBlockHash');
  return this.client.getBestBlockHash();
};
