const cbor = require('cbor');
const Long = require('long');

const ChainInfo = require('./ChainInfo');
const LevelDbTransaction = require('../levelDb/LevelDBTransaction');

class ChainInfoExternalStoreRepository {
  /**
   *
   * @param {LevelUP} externalLevelDB
   */
  constructor(externalLevelDB) {
    this.db = externalLevelDB;
  }

  /**
   * Store chain info
   *
   * @param {ChainInfo} chainInfo
   * @param {LevelDBTransaction} transaction
   * @return {this}
   */
  async store(chainInfo, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    await db.put(
      ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
      cbor.encodeCanonical(chainInfo.toJSON()),
      // transaction-level convert value to string without this flag
      { asBuffer: true },
    );

    return this;
  }

  /**
   * Fetch chain info
   *
   * @param {LevelDBTransaction} [transaction]
   *
   * @return {ChainInfo}
   */
  async fetch(transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    try {
      const chainInfoEncoded = await db.get(
        ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
      );

      if (!chainInfoEncoded) {
        return new ChainInfo();
      }

      const {
        lastBlockHeight,
        lastCoreChainLockedHeight,
      } = cbor.decode(chainInfoEncoded);

      return new ChainInfo(
        Long.fromString(lastBlockHeight),
        lastCoreChainLockedHeight,
      );
    } catch (e) {
      if (e.type === 'NotFoundError') {
        return new ChainInfo();
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

ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME = Buffer.from('chainInfo');

module.exports = ChainInfoExternalStoreRepository;
