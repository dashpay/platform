/**
 * @param {Buffer} publicKeyHash
 * @return {Promise<Buffer>}
 */
export default async function getIdentityByPublicKeyHash(publicKeyHash) {
  const response = await this.client.platform.getIdentityByPublicKeyHash(
    publicKeyHash,
  );

  return response.getIdentity();
};
