const AbstractBasicError = require('../AbstractBasicError');

class DataContractHaveNewIndexWithOldPropertiesError extends AbstractBasicError {
  /**
   * @param {string} documentType
   * @param {string} indexName
   */
  constructor(documentType, indexName) {
    super(`Old properties in the new indices should be defined in the beginning of it. Document with type ${documentType} has new index "${indexName}" with old properties in the wrong order.`);

    this.documentType = documentType;
    this.indexName = indexName;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get document type with changed indices
   *
   * @returns {string}
   */
  getDocumentType() {
    return this.documentType;
  }

  /**
   * Get index name that have old properties
   *
   * @returns {string}
   */
  getIndexName() {
    return this.indexName;
  }
}

module.exports = DataContractHaveNewIndexWithOldPropertiesError;
