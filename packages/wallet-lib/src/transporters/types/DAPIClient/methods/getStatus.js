const logger = require('../../../../logger');

module.exports = async function getStatus() {
  logger.silly('DAPIClient.getStatus');
  return this.client.getStatus();
};
