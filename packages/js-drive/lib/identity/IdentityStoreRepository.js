class IdentityStoreRepository {
  /**
   *
   * @param {MerkDbStore} identitiesStore
   * @param {DashPlatformProtocol} noStateDpp
   */
  constructor(identitiesStore, noStateDpp) {
    this.storage = identitiesStore;
    this.dpp = noStateDpp;
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

    return this.dpp.identity.createFromBuffer(
      encodedIdentity,
      { skipValidation: true },
    );
  }
}

module.exports = IdentityStoreRepository;
