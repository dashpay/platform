const Identity = require('@dashevo/dpp/lib/identity/Identity');

class IdentityStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {decodeProtocolEntity} decodeProtocolEntity
   */
  constructor(groveDBStore, decodeProtocolEntity) {
    this.storage = groveDBStore;
    this.decodeProtocolEntity = decodeProtocolEntity;
  }

  /**
   * Store identity into database
   *
   * @param {Identity} identity
   * @param {GroveDBTransaction} [transaction]
   * @return {Promise<IdentityStoreRepository>}
   */
  async store(identity, transaction = undefined) {
    await this.storage.put(
      IdentityStoreRepository.TREE_PATH,
      identity.getId().toBuffer(),
      identity.toBuffer(),
      { transaction },
    );

    return this;
  }

  /**
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {GroveDBTransaction} [transaction]
   * @return {Promise<null|Identity>}
   */
  async fetch(id, transaction = undefined) {
    const encodedIdentity = this.storage.get(
      IdentityStoreRepository.TREE_PATH,
      id.toBuffer(),
      { transaction },
    );

    if (!encodedIdentity) {
      return null;
    }

    const [, rawIdentity] = this.decodeProtocolEntity(encodedIdentity);

    return new Identity(rawIdentity);
  }

  /**
   * @return {Promise<IdentityStoreRepository>}
   */
  async createTree() {
    await this.storage.createTree([], IdentityStoreRepository.TREE_PATH[0]);

    return this;
  }
}

IdentityStoreRepository.TREE_PATH = [Buffer.from('identities')];

module.exports = IdentityStoreRepository;
