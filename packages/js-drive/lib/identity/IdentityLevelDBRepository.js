const LevelDbTransaction = require('../levelDb/LevelDBTransaction');

class IdentityLevelDBRepository {
  /**
   *
   * @param {LevelUP} identityLevelDB
   * @param {DashPlatformProtocol} noStateDpp
   */
  constructor(identityLevelDB, noStateDpp) {
    this.db = identityLevelDB;
    this.dpp = noStateDpp;
  }

  /**
   * Store identity into database
   *
   * @param {Identity} identity
   * @param {LevelDBTransaction} [transaction]
   * @return {Promise<IdentityLevelDBRepository>}
   */
  async store(identity, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    await db.put(
      identity.getId(),
      identity.toBuffer(),
      // transaction-level convert value to string without this flag
      { asBuffer: true },
    );

    return this;
  }

  /**
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {LevelDBTransaction} [transaction]
   * @return {Promise<null|Identity>}
   */
  async fetch(id, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    try {
      const encodedIdentity = await db.get(id);

      return this.dpp.identity.createFromBuffer(
        encodedIdentity,
        { skipValidation: true },
      );
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

module.exports = IdentityLevelDBRepository;
