const AbstractBasicError = require('../AbstractBasicError');
const Identifier = require('../../../../identifier/Identifier');

class DuplicateDocumentTransitionsWithIdsError extends AbstractBasicError {
  /**
   * @param {
   *   [string, Buffer][]
   * } documentTransitionReferences
   */
  constructor(documentTransitionReferences) {
    const references = documentTransitionReferences
      .map(([type, id]) => `${type} ${Identifier.from(id)}`).join(', ');

    super(`Document transitions with duplicate IDs: ${references}`);

    this.documentTransitionReferences = documentTransitionReferences;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get duplicate transition references
   *
   * @return {
   *   [string, Buffer][]
   * }
   */
  getDocumentTransitionReferences() {
    return this.documentTransitionReferences;
  }
}

module.exports = DuplicateDocumentTransitionsWithIdsError;
