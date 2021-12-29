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
    const identityIds = this.storage.get(publicKeyHash, transaction) || [];

    identityIds.push(identityId);

    this.storage.put(
      publicKeyHash,
      identityIds,
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
   * @return {Promise<Identifier[]>}
   */
  async fetch(publicKeyHash, transaction = undefined) {
    const identityIds = this.storage.get(publicKeyHash, transaction);

    if (identityIds === undefined) {
      return [];
    }

    return identityIds.map((id) => new Identifier(id));
  }
}

module.exports = PublicKeyToIdentityIdStoreRepository;
