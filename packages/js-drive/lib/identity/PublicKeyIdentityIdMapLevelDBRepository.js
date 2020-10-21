const Identifier = require('@dashevo/dpp/lib/Identifier');

const LevelDbTransaction = require('../levelDb/LevelDBTransaction');

class PublicKeyIdentityIdMapLevelDBRepository {
  /**
   *
   * @param {LevelUP} identityLevelDB
   */
  constructor(identityLevelDB) {
    this.db = identityLevelDB;
  }

  /**
   * Store public key to identity id map into database
   *
   * @param {Buffer} publicKeyHash
   * @param {Identifier} identityId
   * @param {LevelDBTransaction} [transaction]
   *
   * @return {Promise<PublicKeyIdentityIdMapLevelDBRepository>}
   */
  async store(publicKeyHash, identityId, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    await db.put(
      publicKeyHash,
      identityId,
      // transaction-level convert value to string without this flag
      { asBuffer: true },
    );

    return this;
  }

  /**
   * Fetch identity id by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {LevelDBTransaction} [transaction]
   *
   * @return {Promise<null|Identifier>}
   */
  async fetch(publicKeyHash, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    try {
      const identityId = await db.get(publicKeyHash);

      return new Identifier(identityId);
    } catch (e) {
      if (e.type === 'NotFoundError') {
        return null;
      }

      throw e;
    }
  }

  /**
   * Creates new transaction instance
   *
   * @return {LevelDBTransaction}
   */
  createTransaction() {
    return new LevelDbTransaction(this.db);
  }
}

module.exports = PublicKeyIdentityIdMapLevelDBRepository;
