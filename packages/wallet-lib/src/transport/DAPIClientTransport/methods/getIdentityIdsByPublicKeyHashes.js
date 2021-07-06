const logger = require('../../../logger');

/**
 * @param {Buffer[]} publicKeyHashes
 * @return {Promise<Buffer[]>}
 */
module.exports = async function getIdentityIdsByPublicKeyHashes(publicKeyHashes) {
  logger.silly('DAPIClientTransport.getIdentityIdsByPublicKeyHashes');

  const response = await this.client.platform.getIdentityIdsByPublicKeyHashes(
    publicKeyHashes,
  );

  return response.getIdentityIds();
};
