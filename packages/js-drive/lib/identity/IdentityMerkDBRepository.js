const MerkDbTransaction = require('../merkDb/MerkDbTransaction');

class IdentityMerkDBRepository {
  /**
   *
   * @param {Merk} identityMerkDB
   * @param {DashPlatformProtocol} noStateDpp
   */
  constructor(identityMerkDB, noStateDpp) {
    this.db = identityMerkDB;
    this.dpp = noStateDpp;
  }

  /**
   * Store identity into database
   *
   * @param {Identity} identity
   * @param {MerkDbTransaction} [transaction]
   * @return {Promise<IdentityMerkDBRepository>}
   */
  async store(identity, transaction = undefined) {
    let db;

    if (transaction) {
      db = transaction.db;

      db.put(
        identity.getId(),
        identity.toBuffer(),
      );
    } else {
      db = this.db;

      db
        .batch()
        .put(
          identity.getId(),
          identity.toBuffer(),
        )
        .commitSync();
    }

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
    const db = transaction ? transaction.db : this.db;

    try {
      const encodedIdentity = db.getSync(id);

      return this.dpp.identity.createFromBuffer(
        encodedIdentity,
        { skipValidation: true },
      );
    } catch (e) {
      if (e.message.startsWith('key not found')) {
        return null;
      }

      throw e;
    }
  }

  /**
   * Creates new transaction instance
   *
   * @return {MerkDbTransaction}
   */
  createTransaction() {
    return new MerkDbTransaction(this.db);
  }
}

module.exports = IdentityMerkDBRepository;
