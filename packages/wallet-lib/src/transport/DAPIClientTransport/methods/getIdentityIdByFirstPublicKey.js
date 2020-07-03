const logger = require('../../../logger');

module.exports = async function getIdentityIdByFirstPublicKey(publicKeyHash) {
  logger.silly('DAPIClientTransport.getIdentityIdByFirstPublicKey');

  return this.client.platform.getIdentityIdByFirstPublicKey(publicKeyHash);
};
