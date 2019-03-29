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
   * @param {boolean} reverting
   * @returns {Promise<void>}
   */
  async function updateSVDocument(contractId, userId, reference, document, reverting) {
    const svDocumentRepository = createSVDocumentRepository(contractId, document.getType());

    const svDocument = new SVDocument(userId, document, reference);

    switch (document.getAction()) {
      case Document.ACTIONS.CREATE: {
        await svDocumentRepository.store(svDocument);

        break;
      }

      case Document.ACTIONS.DELETE: {
        svDocument.markAsDeleted();
      }
      // eslint-disable-next-line no-fallthrough
      case Document.ACTIONS.UPDATE: {
        const previousSVDocument = await svDocumentRepository.find(
          svDocument.getDocument().getId(),
        );

        if (!previousSVDocument) {
          throw new Error('There is no document to update');
        }

        svDocument.addRevision(previousSVDocument);

        // NOTE: Since reverting is more complicated
        // `previousSVDocument` is actually next one here
        // so we have to remove it's revision and the revision before that
        // to have a proper set of `previousRevisions`
        if (reverting) {
          svDocument.removeAheadRevisions();
        }

        await svDocumentRepository.store(svDocument);

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
