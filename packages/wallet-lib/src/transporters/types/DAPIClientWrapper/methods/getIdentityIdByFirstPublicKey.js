const logger = require('../../../../logger');

module.exports = async function getIdentityIdByFirstPublicKey(publicKeyHash) {
  logger.silly('DAPIClientWrapper.getIdentityIdByFirstPublicKey');

  return this.client.getIdentityIdByFirstPublicKey(publicKeyHash);
};
