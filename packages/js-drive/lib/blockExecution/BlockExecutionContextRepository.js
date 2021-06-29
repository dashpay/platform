const cbor = require('cbor');

const LevelDbTransaction = require('../levelDb/LevelDBTransaction');
const BlockExecutionContext = require('./BlockExecutionContext');

class BlockExecutionContextRepository {
  /**
   *
   * @param {LevelUP} externalLevelDB
   */
  constructor(externalLevelDB) {
    this.db = externalLevelDB;
  }

  /**
   * Store block execution context
   *
   * @param {Buffer} keyPrefix
   * @param {BlockExecutionContext} blockExecutionContext
   * @param {LevelDBTransaction} transaction
   * @return {this}
   */
  async store(keyPrefix, blockExecutionContext, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    const object = blockExecutionContext.toObject({ skipConsensusLogger: true });

    await db.put(
      this.makeKey(keyPrefix),
      await cbor.encodeAsync(object),
      // transaction-level convert value to string without this flag
      { asBuffer: true },
    );

    return this;
  }

  /**
   * Fetch chain info
   *
   * @param {Buffer} keyPrefix
   * @param {LevelDBTransaction} [transaction]
   *
   * @return {ChainInfo}
   */
  async fetch(keyPrefix, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    try {
      const blockExecutionContextEncoded = await db.get(
        this.makeKey(keyPrefix),
      );

      const blockExecutionContext = new BlockExecutionContext();

      if (!blockExecutionContextEncoded) {
        return blockExecutionContext;
      }

      blockExecutionContext.fromObject(
        cbor.decode(blockExecutionContextEncoded),
      );

      return blockExecutionContext;
    } catch (e) {
      if (e.type === 'NotFoundError') {
        return new BlockExecutionContext();
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

  /**
   * @private
   * @param {Buffer} keyPrefix
   */
  makeKey(keyPrefix) {
    return Buffer.concat([
      BlockExecutionContextRepository.EXTERNAL_STORE_KEY_NAME,
      keyPrefix,
    ]);
  }
}

BlockExecutionContextRepository.EXTERNAL_STORE_KEY_NAME = Buffer.from('blockExecutionContext');
BlockExecutionContextRepository.KEY_PREFIX_CURRENT = Buffer.from('current');
BlockExecutionContextRepository.KEY_PREFIX_PREVIOUS = Buffer.from('previous');

module.exports = BlockExecutionContextRepository;
