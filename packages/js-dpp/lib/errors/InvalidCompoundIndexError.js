const ConsensusError = require('./ConsensusError');

class InvalidCompoundIndexError extends ConsensusError {
  /**
   *
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(documentType, indexDefinition) {
    super('All or none of unique compound index properties must be set');

    this.documentType = documentType;
    this.indexDefinition = indexDefinition;
  }

  /**
   *
   * @return {Object}
   */
  getIndexDefinition() {
    return this.indexDefinition;
  }

  /**
   *
   * @return {string}
   */
  getDocumentType() {
    return this.documentType;
  }
}

module.exports = InvalidCompoundIndexError;
