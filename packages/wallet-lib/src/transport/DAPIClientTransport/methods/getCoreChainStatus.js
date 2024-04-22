const logger = require('../../../logger');

module.exports = async function getCoreChainStatus() {
  logger.silly('DAPIClientTransport.getCoreChainStatus');

  return this.client.core.getCoreChainStatus();
};
