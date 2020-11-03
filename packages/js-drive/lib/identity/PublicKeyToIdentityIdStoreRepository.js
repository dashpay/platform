const Identifier = require('@dashevo/dpp/lib/Identifier');

class PublicKeyToIdentityIdStoreRepository {
  /**
   *
   * @param {MerkDbStore} publicKeyToIdentityIdStore
   */
  constructor(publicKeyToIdentityIdStore) {
    this.storage = publicKeyToIdentityIdStore;
  }

  /**
   * Store public key to identity id map into database
   *
   * @param {Buffer} publicKeyHash
   * @param {Identifier} identityId
   * @param {MerkDbTransaction} [transaction]
   *
   * @return {Promise<PublicKeyToIdentityIdStoreRepository>}
   */
  async store(publicKeyHash, identityId, transaction = undefined) {
    this.storage.put(
      publicKeyHash,
      identityId,
      transaction,
    );

    return this;
  }

  /**
   * Fetch identity id by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {MerkDbTransaction} [transaction]
   *
   * @return {Promise<null|Identifier>}
   */
  async fetch(publicKeyHash, transaction = undefined) {
    const identityId = this.storage.get(publicKeyHash, transaction);

    if (!identityId) {
      return null;
    }

    return new Identifier(identityId);
  }
}

module.exports = PublicKeyToIdentityIdStoreRepository;
