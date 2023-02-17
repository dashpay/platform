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
     * @param {Object<string, Buffer>} transactionIdMap
     * @param {Object} options
     *
     * @returns {Promise<void>}
     */
  async function updateWithdrawalTransactionIdAndStatus(
    blockInfo,
    coreChainLockedHeight,
    transactionIdMap,
    options,
  ) {
    const originalTransactionIds = Object.keys(transactionIdMap).map((key) => Buffer.from(key, 'hex'));

    const fetchOptions = {
      where: [
        ['status', '==', WITHDRAWALS_STATUS_POOLED],
        ['transactionId', 'in', originalTransactionIds],
      ],
      ...options,
    };

    const documents = await fetchDocuments(
      withdrawalsContractId,
      WITHDRAWALS_DOCUMENT_TYPE,
      fetchOptions,
    );

    for (const document of documents) {
      const originalTransactionIdHex = document.get('transactionId').toString('hex');

      const updatedTransactionId = transactionIdMap[originalTransactionIdHex];

      document.set('transactionId', updatedTransactionId);
      document.set('transactionSignHeight', coreChainLockedHeight);
      document.set('status', WITHDRAWALS_STATUS_BROADCASTED);
      document.setRevision(document.getRevision() + 1);

      await documentRepository.update(document, blockInfo, options);
    }
  }

  return updateWithdrawalTransactionIdAndStatus;
}

module.exports = updateWithdrawalTransactionIdAndStatusFactory;
