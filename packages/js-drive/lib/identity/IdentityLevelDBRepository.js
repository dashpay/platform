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
      this.addKeyPrefix(identity.getId()),
      identity.serialize(),
      { asBuffer: true },
    );

    return this;
  }

  /**
   * Fetch identity by id from database
   *
   * @param {string} id
   * @param {LevelDBTransaction} [transaction]
   * @return {Promise<null|Identity>}
   */
  async fetch(id, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    try {
      const encodedIdentity = await db.get(
        this.addKeyPrefix(id),
      );

      return this.dpp.identity.createFromSerialized(
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
   * Get DB key by identity id
   *
   * @private
   * @param {string} id
   * @return {string}
   */
  addKeyPrefix(id) {
    return `${IdentityLevelDBRepository.KEY_PREFIX}:${id}`;
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

IdentityLevelDBRepository.KEY_PREFIX = 'identity';

module.exports = IdentityLevelDBRepository;
