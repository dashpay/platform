const GetPacketTimeoutError = require('../../storage/errors/GetPacketTimeoutError');

const ReaderMediator = require('../../blockchain/reader/BlockchainReaderMediator');

const rejectAfter = require('../../util/rejectAfter');

/**
 *
 * @param {IpfsAPI} ipfsAPI
 * @param {RpcClient} rpcClient
 * @param {createDapObjectMongoDbRepository} createDapObjectMongoDbRepository
 * @param {applyStateTransition} applyStateTransition
 * @param [applyStateTransitionFromReference} applyStateTransitionFromReference
 * @param {BlockchainReaderMediator} readerMediator
 * @param {number} ipfsGetTimeout
 * @returns {revertDapObjectsForStateTransition}
 */
module.exports = function revertDapObjectsForStateTransitionFactory(
  ipfsAPI,
  rpcClient,
  createDapObjectMongoDbRepository,
  applyStateTransition,
  applyStateTransitionFromReference,
  readerMediator,
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
      const previousRevisions = dapObject.getPreviousRevisions();

      if (previousRevisions.length === 0) {
        dapObject.markAsDeleted();
        await dapObjectMongoDbRepository.store(dapObject);

        await readerMediator.emitSerial(ReaderMediator.EVENTS.DAP_OBJECT_MARKED_DELETED, {
          userId: stateTransition.extraPayload.regTxId,
          objectId: dapObject.getId(),
          reference: dapObject.reference,
          object: dapObject.getOriginalData(),
        });

        continue;
      }

      const [lastPreviousRevision] = previousRevisions
        .sort((prev, next) => next.revision - prev.revision);
      await applyStateTransitionFromReference(lastPreviousRevision.reference, true);

      await readerMediator.emitSerial(ReaderMediator.EVENTS.DAP_OBJECT_REVERTED, {
        userId: stateTransition.extraPayload.regTxId,
        objectId: dapObject.getId(),
        reference: dapObject.reference,
        object: dapObject.getOriginalData(),
        previousRevision: lastPreviousRevision,
      });
    }
  }

  return revertDapObjectsForStateTransition;
};
