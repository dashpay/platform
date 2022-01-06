const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const Identity = require('@dashevo/dpp/lib/identity/Identity');

const decodeProtocolEntity = decodeProtocolEntityFactory();

class IdentityStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store identity into database
   *
   * @param {Identity} identity
   * @param {GroveDBTransaction} [transaction]
   * @return {Promise<IdentityStoreRepository>}
   */
  async store(identity, transaction = undefined) {
    this.storage.put(
      identity.getId().toBuffer(),
      identity.toBuffer(),
      transaction,
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
    const encodedIdentity = this.storage.get(id.toBuffer(), transaction);

    if (!encodedIdentity) {
      return null;
    }

    const [, rawIdentity] = decodeProtocolEntity(encodedIdentity);

    return new Identity(rawIdentity);
  }
}

module.exports = IdentityStoreRepository;
