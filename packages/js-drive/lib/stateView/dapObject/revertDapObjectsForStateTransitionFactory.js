const GetPacketTimeoutError = require('../../storage/errors/GetPacketTimeoutError');

const rejectAfter = require('../../util/rejectAfter');

/**
 *
 * @param {IpfsAPI} ipfsAPI
 * @param {RpcClient} rpcClient
 * @param {createDapObjectMongoDbRepository} createDapObjectMongoDbRepository
 * @param {applyStateTransition} applyStateTransition
 * @param [applyStateTransitionFromReference} applyStateTransitionFromReference
 * @param {number} ipfsGetTimeout
 * @returns {revertDapObjectsForStateTransition}
 */
module.exports = function revertDapObjectsForStateTransitionFactory(
  ipfsAPI,
  rpcClient,
  createDapObjectMongoDbRepository,
  applyStateTransition,
  applyStateTransitionFromReference,
  ipfsGetTimeout,
) {
  /**
   * @typedef revertDapObjectsForStateTransition
   * @param {StateTransitionHeader} stateTransition
   * @returns {Promise<void>}
   */
  async function revertDapObjectsForStateTransition({ stateTransition }) {
    const getPacketDataPromise = ipfsAPI.dag.get(stateTransition.getPacketCID());
    const error = new GetPacketTimeoutError();
    const { value: packetData } = await rejectAfter(getPacketDataPromise, error, ipfsGetTimeout);

    if (!packetData.dapid) {
      return;
    }

    const dapObjectMongoDbRepository = await createDapObjectMongoDbRepository(packetData.dapid);

    const dapObjects = await dapObjectMongoDbRepository
      .findAllBySTHeaderHash(stateTransition.hash);

    for (const dapObject of dapObjects) {
      const previousRevisions = dapObject.getPreviousRevisions()
        .sort((prev, next) => prev.revision - next.revision);

      if (previousRevisions.length === 0) {
        dapObject.markAsDeleted();
        await dapObjectMongoDbRepository.store(dapObject);

        continue;
      }

      for (const { reference } of previousRevisions) {
        await applyStateTransitionFromReference(reference);
      }
    }
  }

  return revertDapObjectsForStateTransition;
};
