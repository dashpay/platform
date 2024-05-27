const logger = require('../../../logger');

module.exports = async function getBestBlockHeight() {
  logger.silly('DAPIClientTransport.getBestBlockHeight');

  return this.client.core.getBestBlockHeight();
};
