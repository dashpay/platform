const WITHDRAWALS_DOCUMENT_TYPE = 'withdrawals';

const WITHDRAWALS_STATUS_POOLED = 1;
const WITHDRAWALS_STATUS_BROADCASTED = 2;

/**
 * @param {DocumentRepository} documentRepository
 * @param {fetchDocuments} fetchDocuments
 * @param {Identifier} withdrawalsContractId
 *
 * @returns {updateWithdrawalTransactionIdAndStatus}
 */
function updateWithdrawalTransactionIdAndStatusFactory(
  documentRepository,
  fetchDocuments,
  withdrawalsContractId,
) {
  /**
     * Update withdrawal transactionId and set status to BROADCASTED
     *
     * @typedef updateWithdrawalTransactionIdAndStatus
     *
     * @param {BlockInfo} blockInfo
     * @param {number} coreChainLockedHeight
     * @param {Buffer} originalTransactionId
     * @param {Buffer} updatedTransactionId
     * @param {Object} options
     *
     * @returns {Promise<void>}
     */
  async function updateWithdrawalTransactionIdAndStatus(
    blockInfo,
    coreChainLockedHeight,
    originalTransactionId,
    updatedTransactionId,
    options,
  ) {
    const fetchOptions = {
      where: [
        ['status', '==', WITHDRAWALS_STATUS_POOLED],
        ['transactionId', '==', originalTransactionId],
      ],
      ...options,
    };

    const documents = await fetchDocuments(
      withdrawalsContractId,
      WITHDRAWALS_DOCUMENT_TYPE,
      fetchOptions,
    );

    for (const document of documents) {
      document.set('transactionSignHeight', coreChainLockedHeight);
      document.set('transactionId', updatedTransactionId);
      document.set('status', WITHDRAWALS_STATUS_BROADCASTED);
      document.setRevision(document.getRevision() + 1);

      await documentRepository.update(document, blockInfo, options);
    }
  }

  return updateWithdrawalTransactionIdAndStatus;
}

module.exports = updateWithdrawalTransactionIdAndStatusFactory;
