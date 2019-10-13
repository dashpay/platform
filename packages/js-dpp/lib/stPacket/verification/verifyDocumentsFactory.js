const Document = require('../../document/Document');

const ValidationResult = require('../../validation/ValidationResult');

const InvalidDocumentActionError = require('../errors/InvalidDocumentActionError');

const DocumentAlreadyPresentError = require('../../errors/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../errors/DocumentNotFoundError');
const InvalidDocumentRevisionError = require('../../errors/InvalidDocumentRevisionError');
const InvalidDocumentScopeError = require('../../errors/InvalidDocumentScopeError');
const ContractNotPresentError = require('../../errors/DataContractNotPresentError');

const hash = require('../../util/hash');

/**
 * @param {fetchDocumentsByDocuments} fetchDocumentsByDocuments
 * @param {verifyDocumentsUniquenessByIndices} verifyDocumentsUniquenessByIndices
 * @return {verifyDocuments}
 */
function verifyDocumentsFactory(fetchDocumentsByDocuments, verifyDocumentsUniquenessByIndices) {
  /**
   * @typedef verifyDocuments
   * @param {STPacket} stPacket
   * @param {string} userId
   * @param {Contract} contract
   * @return {ValidationResult}
   */
  async function verifyDocuments(stPacket, userId, contract) {
    const result = new ValidationResult();

    if (!contract) {
      result.addError(
        new ContractNotPresentError(stPacket.getContractId()),
      );

      return result;
    }

    const fetchedDocuments = await fetchDocumentsByDocuments(
      stPacket.getContractId(),
      stPacket.getDocuments(),
    );

    stPacket.getDocuments()
      .forEach((document) => {
        const fetchedDocument = fetchedDocuments.find(o => document.getId() === o.getId());

        const stPacketScope = hash(stPacket.getContractId() + userId).toString('hex');
        if (document.scope !== stPacketScope) {
          result.addError(
            new InvalidDocumentScopeError(document),
          );
        }

        switch (document.getAction()) {
          case Document.ACTIONS.CREATE:
            if (fetchedDocument) {
              result.addError(
                new DocumentAlreadyPresentError(document, fetchedDocument),
              );
            }
            break;
          case Document.ACTIONS.UPDATE:
          case Document.ACTIONS.DELETE:
            if (!fetchedDocument) {
              result.addError(
                new DocumentNotFoundError(document),
              );

              break;
            }

            if (document.getRevision() !== fetchedDocument.getRevision() + 1) {
              result.addError(
                new InvalidDocumentRevisionError(document, fetchedDocument),
              );
            }

            break;
          default:
            throw new InvalidDocumentActionError(document);
        }
      });

    result.merge(
      await verifyDocumentsUniquenessByIndices(stPacket, userId, contract),
    );

    return result;
  }

  return verifyDocuments;
}

module.exports = verifyDocumentsFactory;
