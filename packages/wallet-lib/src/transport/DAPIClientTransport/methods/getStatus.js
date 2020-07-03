const logger = require('../../../logger');

module.exports = async function getStatus() {
  logger.silly('DAPIClientTransport.getStatus');
  return this.client.core.getStatus();
};
