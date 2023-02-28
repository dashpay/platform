const BlockInfo = require('../../../blockExecution/BlockInfo');

/**
 * @param {CoreRpcClient} coreRpcClient
 * @param {updateWithdrawalTransactionIdAndStatus} updateWithdrawalTransactionIdAndStatus
 *
 * @return {broadcastWithdrawalTransactions}
 */
function broadcastWithdrawalTransactionsFactory(
  coreRpcClient,
  updateWithdrawalTransactionIdAndStatus,
) {
  /**
   * @typedef broadcastWithdrawalTransactions
   *
   * @param {BlockExecutionContext} proposalBlockExecutionContext
   * @param {{{ extension: Buffer, signature: Buffer }}[]} thresholdVoteExtensions
   * @param {Object} unsignedWithdrawalTransactionsMap
   *
   * @return {Promise<void>}
   */
  async function broadcastWithdrawalTransactions(
    proposalBlockExecutionContext,
    thresholdVoteExtensions,
    unsignedWithdrawalTransactionsMap,
  ) {
    const blockInfo = BlockInfo.createFromBlockExecutionContext(proposalBlockExecutionContext);

    const transactionIdMap = {};

    for (const { extension, signature } of (thresholdVoteExtensions || [])) {
      const withdrawalTransactionHash = extension.toString('hex');

      const unsignedWithdrawalTransactionBytes = unsignedWithdrawalTransactionsMap[
        withdrawalTransactionHash
      ];

      if (unsignedWithdrawalTransactionBytes) {
        const transactionBytes = Buffer.concat([
          unsignedWithdrawalTransactionBytes,
          signature,
        ]);

        transactionIdMap[unsignedWithdrawalTransactionBytes.toString('hex')] = transactionBytes;

        // TODO: think about Core error handling
        await coreRpcClient.sendRawTransaction(transactionBytes.toString('hex'));
      }
    }

    await updateWithdrawalTransactionIdAndStatus(
      blockInfo,
      proposalBlockExecutionContext.getCoreChainLockedHeight(),
      transactionIdMap,
      {
        useTransaction: true,
      },
    );
  }

  return broadcastWithdrawalTransactions;
}

module.exports = broadcastWithdrawalTransactionsFactory;
