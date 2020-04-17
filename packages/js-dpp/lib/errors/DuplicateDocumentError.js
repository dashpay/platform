const ConsensusError = require('./ConsensusError');

class DuplicateDocumentError extends ConsensusError {
  /**
   * @param {DocumentCreateTransition|DocumentReplaceTransition} documentTransition
   * @param {Object} indexDefinition
   */
  constructor(documentTransition, indexDefinition) {
    super('Duplicate Document found');

    this.documentTransition = documentTransition;
    this.indexDefinition = indexDefinition;
  }

  /**
   * Get document action transition
   *
   * @return {DocumentCreateTransition|DocumentReplaceTransition}
   */
  getDocumentTransition() {
    return this.documentTransition;
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
