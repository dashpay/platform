/**
 * @param {DapContractMongoDbRepository} dapContractMongoDbRepository
 * @param {RpcClient} rpcClient
 * @param {applyStateTransition} applyStateTransition
 * @param {applyStateTransitionFromReference} applyStateTransitionFromReference
 * @returns {revertDapContractsForStateTransition}
 */
function revertDapContractsForStateTransitionFactory(
  dapContractMongoDbRepository,
  rpcClient,
  applyStateTransition,
  applyStateTransitionFromReference,
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

        continue;
      }

      for (const { reference } of previousVersions) {
        await applyStateTransitionFromReference(reference);
      }
    }
  }

  return revertDapContractsForStateTransition;
}

module.exports = revertDapContractsForStateTransitionFactory;
