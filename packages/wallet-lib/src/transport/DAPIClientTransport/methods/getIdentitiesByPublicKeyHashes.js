const logger = require('../../../logger');

/**
 * @param {Buffer[]} publicKeyHashes
 * @return {Promise<Buffer[]>}
 */
module.exports = async function getIdentitiesByPublicKeyHashes(publicKeyHashes) {
  logger.silly('DAPIClientTransport.getIdentitiesByPublicKeyHashes');

  const response = await this.client.platform.getIdentitiesByPublicKeyHashes(
    publicKeyHashes,
  );

  return response.getIdentities();
};
