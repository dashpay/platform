const ReaderMediator = require('../../blockchain/reader/BlockchainReaderMediator');

/**
 *
 * @param {STPacketIpfsRepository} stPacketRepository
 * @param {RpcClient} rpcClient
 * @param {createSVDocumentMongoDbRepository} createSVDocumentRepository
 * @param {applyStateTransition} applyStateTransition
 * @param [applyStateTransitionFromReference} applyStateTransitionFromReference
 * @param {BlockchainReaderMediator} readerMediator
 * @returns {revertSVDocumentsForStateTransition}
 */
module.exports = function revertSVDocumentsForStateTransitionFactory(
  stPacketRepository,
  rpcClient,
  createSVDocumentRepository,
  applyStateTransition,
  applyStateTransitionFromReference,
  readerMediator,
) {
  /**
   * @typedef revertSVDocumentsForStateTransition
   * @param {StateTransition} stateTransition
   * @returns {Promise<void>}
   */
  async function revertSVDocumentsForStateTransition({ stateTransition }) {
    const stPacket = await stPacketRepository.find(stateTransition.getPacketCID());

    const documentTypes = new Set(stPacket.getDocuments().map(d => d.getType()));

    for (const documentType of documentTypes) {
      const svDocumentRepository = await createSVDocumentRepository(
        stPacket.getContractId(),
        documentType,
      );

      const svDocuments = await svDocumentRepository.findAllBySTHash(stateTransition.hash);

      for (const svDocument of svDocuments) {
        const previousRevisions = svDocument.getPreviousRevisions();

        if (previousRevisions.length === 0) {
          svDocument.markAsDeleted();

          await svDocumentRepository.store(svDocument);

          await readerMediator.emitSerial(ReaderMediator.EVENTS.DOCUMENT_MARKED_DELETED, {
            userId: stateTransition.extraPayload.regTxId,
            documentId: svDocument.getDocument().getId(),
            reference: svDocument.getReference(),
            document: svDocument.getDocument().toJSON(),
          });

          continue;
        }

        const [lastPreviousRevision] = previousRevisions
          .sort((prev, next) => next.revision - prev.revision);

        await applyStateTransitionFromReference(lastPreviousRevision.getReference(), true);

        await readerMediator.emitSerial(ReaderMediator.EVENTS.DOCUMENT_REVERTED, {
          userId: svDocument.getUserId(),
          documentId: svDocument.getDocument().getId(),
          reference: svDocument.getReference(),
          document: svDocument.getDocument().toJSON(),
          previousRevision: lastPreviousRevision,
        });
      }
    }
  }

  return revertSVDocumentsForStateTransition;
};
