/**
 * @param {Buffer} publicKeyHash
 * @return {Promise<Buffer>}
 */
module.exports = async function getIdentityByPublicKeyHash(publicKeyHash) {
  const response = await this.client.platform.getIdentityByPublicKeyHash(
    publicKeyHash,
  );

  return response.getIdentity();
};
