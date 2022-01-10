const cbor = require('cbor');

const BlockExecutionContextStack = require('./BlockExecutionContextStack');

class BlockExecutionContextStackRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.db = groveDBStore;
  }

  /**
   * Store block execution context
   *
   * @param {BlockExecutionContextStack} blockExecutionContextStack
   * @param {GroveDBTransaction} transaction
   * @return {this}
   */
  async store(blockExecutionContextStack, transaction = undefined) {
    const contexts = blockExecutionContextStack.getContexts()
      .map((context) => context.toObject({
        skipDBTransaction: true,
        skipConsensusLogger: true,
      }));

    await this.db.putAux(
      BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
      await cbor.encodeAsync(contexts),
      { transaction },
    );

    return this;
  }

  /**
   * Fetch block execution stack
   *
   * @param {GroveDBTransaction} [transaction]
   *
   * @return {BlockExecutionContextStack}
   */
  async fetch(transaction = undefined) {
    try {
      const blockExecutionContextsEncoded = await this.db.getAux(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
        { transaction },
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
}

BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME = Buffer.from('blockExecutionContext');

module.exports = BlockExecutionContextStackRepository;
