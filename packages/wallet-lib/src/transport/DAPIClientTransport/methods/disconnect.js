const logger = require('../../../logger');

module.exports = async function disconnect() {
  logger.silly('DAPIClientTransport.disconnect');

  return this.client.disconnect();
};
