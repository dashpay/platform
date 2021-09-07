const AbstractStateError = require('../AbstractStateError');
const Identifier = require('../../../../identifier/Identifier');

class DocumentOwnerIdMismatchError extends AbstractStateError {
  /**
   * @param {Buffer} documentId
   * @param {Buffer} documentOwnerId
   * @param {Buffer} existingDocumentOwnerId
   */
  constructor(documentId, documentOwnerId, existingDocumentOwnerId) {
    super(`Provided document ${Identifier.from(documentId)} owner ID ${Identifier.from(documentOwnerId)} mismatch with existing ${Identifier.from(existingDocumentOwnerId)}`);

    this.documentId = documentId;
    this.documentOwnerId = documentOwnerId;
    this.existingDocumentOwnerId = existingDocumentOwnerId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get document ID
   *
   * @returns {Buffer}
   */
  getDocumentId() {
    return this.documentId;
  }

  /**
   * Get document owner ID
   *
   * @return {Buffer}
   */
  getDocumentOwnerId() {
    return this.documentOwnerId;
  }

  /**
   * Get existing Document owner ID
   *
   * @return {Buffer}
   */
  getExistingDocumentOwnerId() {
    return this.existingDocumentOwnerId;
  }
}

module.exports = DocumentOwnerIdMismatchError;
