const ConsensusError = require('./ConsensusError');

class DuplicateDocumentError extends ConsensusError {
  /**
   * @param {Document} document
   * @param {Object} indexDefinition
   */
  constructor(document, indexDefinition) {
    super('Duplicate Document found');

    this.document = document;
    this.indexDefinition = indexDefinition;
  }

  /**
   * Get Document
   *
   * @return {Document}
   */
  getDocument() {
    return this.document;
  }

  /**
   * Get index definition
   *
   * @return {Object}
   */
  getIndexDefinition() {
    return this.indexDefinition;
  }
}

module.exports = DuplicateDocumentError;
