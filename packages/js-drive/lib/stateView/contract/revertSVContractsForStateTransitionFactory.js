const ReaderMediator = require('../../../lib/blockchain/reader/BlockchainReaderMediator');

/**
 * @param {SVContractMongoDbRepository} svContractRepository
 * @param {RpcClient} rpcClient
 * @param {applyStateTransition} applyStateTransition
 * @param {applyStateTransitionFromReference} applyStateTransitionFromReference
 * @param {BlockchainReaderMediator} readerMediator
 * @returns {revertSVContractsForStateTransition}
 */
function revertSVContractsForStateTransitionFactory(
  svContractRepository,
  rpcClient,
  applyStateTransition,
  applyStateTransitionFromReference,
  readerMediator,
) {
  /**
   * @typedef revertSVContractsForStateTransition
   * @param {{ stateTransition: StateTransition, block: object }}
   * @returns {Promise<void>}
   */
  async function revertSVContractsForStateTransition({ stateTransition }) {
    const svContracts = await svContractRepository
      .findAllByReferenceSTHash(stateTransition.hash);

    for (const svContract of svContracts) {
      const previousRevisions = svContract.getPreviousRevisions();

      if (previousRevisions.length === 0) {
        svContract.markAsDeleted();

        await svContractRepository.store(svContract);

        await readerMediator.emitSerial(ReaderMediator.EVENTS.CONTRACT_MARKED_DELETED, {
          userId: stateTransition.extraPayload.regTxId,
          contractId: svContract.getContractId(),
          reference: svContract.getReference(),
          contract: svContract.getContract().toJSON(),
        });

        continue;
      }

      const [lastPreviousRevision] = previousRevisions
        .sort((prev, next) => next.getRevision() - prev.getRevision());

      await applyStateTransitionFromReference(lastPreviousRevision.getReference(), true);

      await readerMediator.emitSerial(ReaderMediator.EVENTS.CONTRACT_REVERTED, {
        userId: stateTransition.extraPayload.regTxId,
        contractId: svContract.getContractId(),
        reference: svContract.getReference(),
        contract: svContract.getContract().toJSON(),
        previousRevision: lastPreviousRevision,
      });
    }
  }

  return revertSVContractsForStateTransition;
}

module.exports = revertSVContractsForStateTransitionFactory;
