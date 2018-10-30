const StateTransitionHeader = require('./StateTransitionHeader');

/**
 * @param {RpcClient} rpcClient
 * @return {createStateTransitionsFromBlock}
 */
module.exports = function createStateTransitionsFromBlockFactory(rpcClient) {
  /**
   * @typedef createStateTransitionsFromBlock
   * @param {object} block
   * @return {StateTransitionHeader[]}
   */
  async function createStateTransitionsFromBlock(block) {
    const stateTransitions = [];

    for (const transactionId of block.tx) {
      const { result: serializedTransaction } = await rpcClient.getRawTransaction(transactionId);

      const transaction = new StateTransitionHeader(serializedTransaction);

      if (transaction.isSpecialTransaction()
          && transaction.type === StateTransitionHeader.TYPES.TRANSACTION_SUBTX_TRANSITION) {
        stateTransitions.push(transaction);
      }
    }

    return stateTransitions;
  }

  return createStateTransitionsFromBlock;
};
