const Identifier = require('@dashevo/dpp/lib/Identifier');

class PublicKeyToIdentityIdStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store public key to identity id map into database
   *
   * @param {Buffer} publicKeyHash
   * @param {Identifier} identityId
   * @param {GroveDBTransaction} [transaction]
   *
   * @return {Promise<PublicKeyToIdentityIdStoreRepository>}
   */
  async store(publicKeyHash, identityId, transaction = undefined) {
    this.storage.put(
      publicKeyHash,
      identityId.toBuffer(),
      transaction,
    );

    return this;
  }

  /**
   * Fetch identity id by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {GroveDBTransaction} [transaction]
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
