/**
 * @param {Buffer[]} publicKeyHashes
 * @return {Promise<Buffer[]>}
 */
module.exports = async function getIdentitiesByPublicKeyHashes(publicKeyHashes) {
  const response = await this.client.platform.getIdentitiesByPublicKeyHashes(
    publicKeyHashes,
  );

  return response.getIdentities();
};
