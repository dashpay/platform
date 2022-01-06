const cbor = require('cbor');

const LevelDbTransaction = require('../levelDb/LevelDBTransaction');
const BlockExecutionContextStack = require('./BlockExecutionContextStack');

class BlockExecutionContextStackRepository {
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
   * @param {BlockExecutionContextStack} blockExecutionContextStack
   * @param {LevelDBTransaction} transaction
   * @return {this}
   */
  async store(blockExecutionContextStack, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    const contexts = blockExecutionContextStack.getContexts()
      .map((context) => context.toObject({
        skipDBTransaction: true,
        skipConsensusLogger: true,
      }));

    await db.put(
      BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
      await cbor.encodeAsync(contexts),
      // transaction-level convert value to string without this flag
      { asBuffer: true },
    );

    return this;
  }

  /**
   * Fetch block execution stack
   *
   * @param {LevelDBTransaction} [transaction]
   *
   * @return {BlockExecutionContextStack}
   */
  async fetch(transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    try {
      const blockExecutionContextsEncoded = await db.get(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
      );

      const blockExecutionContextStack = new BlockExecutionContextStack();

      if (!blockExecutionContextsEncoded) {
        return blockExecutionContextStack;
      }

      const blockExecutionContexts = cbor.decode(blockExecutionContextsEncoded);

      blockExecutionContextStack.setContexts(blockExecutionContexts);

      return blockExecutionContextStack;
    } catch (e) {
      if (e.type === 'NotFoundError') {
        return new BlockExecutionContextStack();
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

BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME = Buffer.from('blockExecutionContext');

module.exports = BlockExecutionContextStackRepository;
