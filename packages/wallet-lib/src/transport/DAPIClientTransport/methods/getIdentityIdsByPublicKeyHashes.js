const logger = require('../../../logger');

/**
 * @param {Buffer[]} publicKeyHashes
 * @return {Promise<*|string>}
 */
module.exports = async function getIdentityIdsByPublicKeyHashes(publicKeyHashes) {
  logger.silly('DAPIClientTransport.getIdentityIdsByPublicKeyHashes');

  return this.client.platform.getIdentityIdsByPublicKeyHashes(
    publicKeyHashes,
  );
};
