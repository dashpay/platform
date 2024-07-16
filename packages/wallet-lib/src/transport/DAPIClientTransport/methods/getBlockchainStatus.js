const logger = require('../../../logger');

module.exports = async function getBlockchainStatus() {
  logger.silly('DAPIClientTransport.getBlockchainStatus');

  return this.client.core.getBlockchainStatus();
};
