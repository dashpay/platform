const Document = require('@dashevo/dpp/lib/document/Document');
const SVDocument = require('./SVDocument');

function updateSVDocumentFactory(createSVDocumentRepository) {
  /**
   * Generate Document State View
   *
   * @typedef {Promise} updateSVDocument
   * @param {string} contractId
   * @param {string} userId
   * @param {Reference} reference
   * @param {Document} document
   * @param {MongoDBTransaction} transaction
   * @returns {Promise<void>}
   */
  async function updateSVDocument(contractId, userId, reference, document, transaction) {
    const svDocumentRepository = createSVDocumentRepository(contractId, document.getType());

    const svDocument = new SVDocument(userId, document, reference);

    switch (document.getAction()) {
      case Document.ACTIONS.CREATE: {
        await svDocumentRepository.store(svDocument, transaction);

        break;
      }

      case Document.ACTIONS.DELETE: {
        svDocument.markAsDeleted();
      }
      // eslint-disable-next-line no-fallthrough
      case Document.ACTIONS.UPDATE: {
        const previousSVDocument = await svDocumentRepository.find(
          svDocument.getDocument().getId(),
          transaction,
        );

        if (!previousSVDocument) {
          throw new Error('There is no document to update');
        }

        svDocument.addRevision(previousSVDocument);

        await svDocumentRepository.store(svDocument, transaction);

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
