const ReaderMediator = require('../../blockchain/reader/BlockchainReaderMediator');

/**
 *
 * @param {StateTransitionPacketIpfsRepository} stPacketRepository
 * @param {RpcClient} rpcClient
 * @param {createDapObjectMongoDbRepository} createDapObjectMongoDbRepository
 * @param {applyStateTransition} applyStateTransition
 * @param [applyStateTransitionFromReference} applyStateTransitionFromReference
 * @param {BlockchainReaderMediator} readerMediator
 * @returns {revertDapObjectsForStateTransition}
 */
module.exports = function revertDapObjectsForStateTransitionFactory(
  stPacketRepository,
  rpcClient,
  createDapObjectMongoDbRepository,
  applyStateTransition,
  applyStateTransitionFromReference,
  readerMediator,
) {
  /**
   * @typedef revertDapObjectsForStateTransition
   * @param {StateTransitionHeader} stateTransition
   * @returns {Promise<void>}
   */
  async function revertDapObjectsForStateTransition({ stateTransition }) {
    const stPacket = await stPacketRepository
      .find(stateTransition.getPacketCID());
    const packetData = stPacket.toJSON({ skipMeta: true });

    if (!packetData.dapid) {
      return;
    }

    const objectTypes = new Set(packetData.dapobjects.map(o => o.objtype));

    for (const objectType of objectTypes) {
      const dapObjectMongoDbRepository = await createDapObjectMongoDbRepository(
        packetData.dapid,
        objectType,
      );

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
  }

  return revertDapObjectsForStateTransition;
};
