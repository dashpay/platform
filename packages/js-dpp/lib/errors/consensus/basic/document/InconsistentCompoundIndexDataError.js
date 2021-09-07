const AbstractBasicError = require('../AbstractBasicError');

class InconsistentCompoundIndexDataError extends AbstractBasicError {
  /**
   *
   * @param {string} documentType
   * @param {string[]} indexedProperties
   */
  constructor(documentType, indexedProperties) {
    super(`Unique compound index properties ${indexedProperties.join(', ')} are partially set for ${documentType} document`);

    this.documentType = documentType;
    this.indexedProperties = indexedProperties;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @return {string[]}
   */
  getIndexedProperties() {
    return this.indexedProperties;
  }

  /**
   *
   * @return {string}
   */
  getDocumentType() {
    return this.documentType;
  }
}

module.exports = InconsistentCompoundIndexDataError;
