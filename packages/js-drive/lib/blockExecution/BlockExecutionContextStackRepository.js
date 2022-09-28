const cbor = require('cbor');

const BlockExecutionContextStack = require('./BlockExecutionContextStack');
const BlockExecutionContext = require('./BlockExecutionContext');

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
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @return {this}
   */
  async store(blockExecutionContextStack, options = {}) {
    const contexts = blockExecutionContextStack.getContexts()
      .map((context) => context.toObject({
        skipConsensusLogger: true,
      }));

    await this.db.putAux(
      BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
      await cbor.encodeAsync(contexts),
      options,
    );

    return this;
  }

  /**
   * Fetch block execution stack
   *
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   *
   * @return {BlockExecutionContextStack}
   */
  async fetch(options = {}) {
    const blockExecutionContextsEncodedResult = await this.db.getAux(
      BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
      options,
    );

    const blockExecutionContextsEncoded = blockExecutionContextsEncodedResult.getValue();

    const blockExecutionContextStack = new BlockExecutionContextStack();

    if (!blockExecutionContextsEncoded) {
      return blockExecutionContextStack;
    }

    const rawBlockExecutionContexts = cbor.decode(blockExecutionContextsEncoded);

    const blockExecutionContexts = rawBlockExecutionContexts.map((rawContext) => {
      const context = new BlockExecutionContext();

      context.fromObject(rawContext);

      return context;
    });

    blockExecutionContextStack.setContexts(blockExecutionContexts);

    return blockExecutionContextStack;
  }
}

BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME = Buffer.from('blockExecutionContext');

module.exports = BlockExecutionContextStackRepository;
