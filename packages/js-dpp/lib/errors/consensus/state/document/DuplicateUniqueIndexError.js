const AbstractStateError = require('../AbstractStateError');
const Identifier = require('../../../../identifier/Identifier');

class DuplicateUniqueIndexError extends AbstractStateError {
  /**
   * @param {Buffer} documentId
   * @param {string[]} duplicatingProperties
   */
  constructor(documentId, duplicatingProperties) {
    super(`Document ${Identifier.from(documentId)} has duplicate unique properties ${duplicatingProperties.join(', ')} with other documents`);

    this.documentId = documentId;
    this.duplicatingProperties = duplicatingProperties;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get document id
   *
   * @return {Buffer}
   */
  getDocumentId() {
    return this.documentId;
  }

  /**
   * Get duplicating properties
   *
   * @return {string[]}
   */
  getDuplicatingProperties() {
    return this.duplicatingProperties;
  }
}

module.exports = DuplicateUniqueIndexError;
