const logger = require('../../../../logger');

module.exports = async function getStatus() {
  logger.silly('DAPIClientWrapper.getStatus');
  return this.client.getStatus();
};
