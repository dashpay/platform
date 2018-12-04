const ReaderMediator = require('../../../lib/blockchain/reader/BlockchainReaderMediator');

/**
 * @param {DapContractMongoDbRepository} dapContractMongoDbRepository
 * @param {RpcClient} rpcClient
 * @param {applyStateTransition} applyStateTransition
 * @param {applyStateTransitionFromReference} applyStateTransitionFromReference
 * @param {BlockchainReaderMediator} readerMediator
 * @returns {revertDapContractsForStateTransition}
 */
function revertDapContractsForStateTransitionFactory(
  dapContractMongoDbRepository,
  rpcClient,
  applyStateTransition,
  applyStateTransitionFromReference,
  readerMediator,
) {
  /**
   * @typedef revertDapContractsForStateTransition
   * @param {{ stateTransition: StateTransitionHeader, block: object }}
   * @returns {Promise<void>}
   */
  async function revertDapContractsForStateTransition({ stateTransition }) {
    const dapContracts = await dapContractMongoDbRepository
      .findAllByReferenceSTHeaderHash(stateTransition.hash);

    for (const dapContract of dapContracts) {
      const previousVersions = dapContract.getPreviousVersions()
        .sort((prev, next) => prev.version - next.version);

      if (previousVersions.length === 0) {
        dapContract.markAsDeleted();
        await dapContractMongoDbRepository.store(dapContract);

        await readerMediator.emitSerial(ReaderMediator.EVENTS.DAP_CONTRACT_MARKED_DELETED, {
          userId: stateTransition.extraPayload.regTxId,
          dapId: dapContract.dapId,
          reference: dapContract.reference,
          contract: dapContract.getOriginalData(),
        });

        continue;
      }

      for (const { reference } of previousVersions) {
        await applyStateTransitionFromReference(reference);
      }

      await readerMediator.emitSerial(ReaderMediator.EVENTS.DAP_CONTRACT_REVERTED, {
        userId: stateTransition.extraPayload.regTxId,
        dapId: dapContract.dapId,
        reference: dapContract.reference,
        contract: dapContract.getOriginalData(),
        previousVersion: previousVersions[previousVersions.length - 1],
      });
    }
  }

  return revertDapContractsForStateTransition;
}

module.exports = revertDapContractsForStateTransitionFactory;
