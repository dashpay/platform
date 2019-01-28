const ReaderMediator = require('../../blockchain/reader/BlockchainReaderMediator');

/**
 *
 * @param {STPacketIpfsRepository} stPacketRepository
 * @param {RpcClient} rpcClient
 * @param {createSVObjectMongoDbRepository} createSVObjectRepository
 * @param {applyStateTransition} applyStateTransition
 * @param [applyStateTransitionFromReference} applyStateTransitionFromReference
 * @param {BlockchainReaderMediator} readerMediator
 * @returns {revertSVObjectsForStateTransition}
 */
module.exports = function revertSVObjectsForStateTransitionFactory(
  stPacketRepository,
  rpcClient,
  createSVObjectRepository,
  applyStateTransition,
  applyStateTransitionFromReference,
  readerMediator,
) {
  /**
   * @typedef revertSVObjectsForStateTransition
   * @param {StateTransition} stateTransition
   * @returns {Promise<void>}
   */
  async function revertSVObjectsForStateTransition({ stateTransition }) {
    const stPacket = await stPacketRepository.find(stateTransition.getPacketCID());

    const objectTypes = new Set(stPacket.getDPObjects().map(o => o.getType()));

    for (const objectType of objectTypes) {
      const svObjectRepository = await createSVObjectRepository(
        stPacket.getDPContractId(),
        objectType,
      );

      const svObjects = await svObjectRepository.findAllBySTHash(stateTransition.hash);

      for (const svObject of svObjects) {
        const previousRevisions = svObject.getPreviousRevisions();

        if (previousRevisions.length === 0) {
          svObject.markAsDeleted();

          await svObjectRepository.store(svObject);

          await readerMediator.emitSerial(ReaderMediator.EVENTS.DP_OBJECT_MARKED_DELETED, {
            userId: stateTransition.extraPayload.regTxId,
            objectId: svObject.getDPObject().getId(),
            reference: svObject.getReference(),
            object: svObject.getDPObject().toJSON(),
          });

          continue;
        }

        const [lastPreviousRevision] = previousRevisions
          .sort((prev, next) => next.revision - prev.revision);

        await applyStateTransitionFromReference(lastPreviousRevision.getReference(), true);

        await readerMediator.emitSerial(ReaderMediator.EVENTS.DP_OBJECT_REVERTED, {
          userId: svObject.getUserId(),
          objectId: svObject.getDPObject().getId(),
          reference: svObject.getReference(),
          object: svObject.getDPObject().toJSON(),
          previousRevision: lastPreviousRevision,
        });
      }
    }
  }

  return revertSVObjectsForStateTransition;
};
