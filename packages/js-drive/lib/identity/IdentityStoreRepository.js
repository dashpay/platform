class IdentityStoreRepository {
  /**
   *
   * @param {MerkDbStore} identitiesStore
   * @param {AwilixContainer} container
   */
  constructor(identitiesStore, container) {
    this.storage = identitiesStore;
    this.container = container;
  }

  /**
   * Store identity into database
   *
   * @param {Identity} identity
   * @param {MerkDbTransaction} [transaction]
   * @return {Promise<IdentityStoreRepository>}
   */
  async store(identity, transaction = undefined) {
    this.storage.put(
      identity.getId(),
      identity.toBuffer(),
      transaction,
    );

    return this;
  }

  /**
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {MerkDbTransaction} [transaction]
   * @return {Promise<null|Identity>}
   */
  async fetch(id, transaction = undefined) {
    const encodedIdentity = this.storage.get(id, transaction);

    if (!encodedIdentity) {
      return null;
    }

    const dpp = this.container.resolve(transaction ? 'transactionalDpp' : 'dpp');

    return dpp.identity.createFromBuffer(
      encodedIdentity,
      { skipValidation: true },
    );
  }
}

module.exports = IdentityStoreRepository;
