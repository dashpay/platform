const AbstractBasicError = require('../AbstractBasicError');

class DuplicateIndexNameError extends AbstractBasicError {
  /**
   * @param {string} documentType
   * @param {string} duplicateIndexName
   */
  constructor(documentType, duplicateIndexName) {
    const message = `Duplicate index name "${duplicateIndexName}" defined in "${documentType}" document`;

    super(message);

    this.documentType = documentType;
    this.duplicateIndexName = duplicateIndexName;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get Document type
   *
   * @return {string}
   */
  getDocumentType() {
    return this.documentType;
  }

  /**
   * Get duplicate index name
   *
   * @returns {string}
   */
  getDuplicateIndexName() {
    return this.duplicateIndexName;
  }
}

module.exports = DuplicateIndexNameError;
