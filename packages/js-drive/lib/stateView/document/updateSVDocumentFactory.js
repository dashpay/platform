const Document = require('@dashevo/dpp/lib/document/Document');
const SVDocument = require('./SVDocument');

function updateSVDocumentFactory(createSVDocumentRepository) {
  /**
   * Generate Document State View
   *
   * @typedef {Promise} updateSVDocument
   * @param {Document} document
   * @param {Reference} reference
   * @param {MongoDBTransaction} stateViewTransaction
   * @returns {Promise<void>}
   */
  async function updateSVDocument(document, reference, stateViewTransaction) {
    const svDocumentRepository = createSVDocumentRepository(
      document.getDataContractId(),
      document.getType(),
    );

    const svDocument = new SVDocument(document, reference);

    switch (document.getAction()) {
      case Document.ACTIONS.CREATE: {
        await svDocumentRepository.store(svDocument, stateViewTransaction);

        break;
      }

      case Document.ACTIONS.DELETE: {
        svDocument.markAsDeleted();
      }

      // eslint-disable-next-line no-fallthrough
      case Document.ACTIONS.UPDATE: {
        const previousSVDocument = await svDocumentRepository.find(
          svDocument.getDocument().getId(),
          stateViewTransaction,
        );

        if (!previousSVDocument) {
          throw new Error('There is no document to update');
        }

        svDocument.addRevision(previousSVDocument);

        await svDocumentRepository.store(svDocument, stateViewTransaction);

        break;
      }

      default: {
        throw new Error('Unsupported action');
      }
    }
  }

  return updateSVDocument;
}

module.exports = updateSVDocumentFactory;
