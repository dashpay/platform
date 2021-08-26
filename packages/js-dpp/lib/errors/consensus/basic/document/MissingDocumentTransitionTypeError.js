const AbstractBasicError = require('../AbstractBasicError');

class MissingDocumentTransitionTypeError extends AbstractBasicError {
  /**
   * @param {
   *    RawDocumentCreateTransition|
   *    RawDocumentReplaceTransition|
   *    RawDocumentDeleteTransition
   * } rawDocumentTransition
   */
  constructor(rawDocumentTransition) {
    super('$type is not present');

    this.rawDocumentTransition = rawDocumentTransition;
  }

  /**
   * Get Raw Document Transition
   *
   * @return {
   *    RawDocumentCreateTransition|
   *    RawDocumentReplaceTransition|
   *    RawDocumentDeleteTransition
   * }
   */
  getRawDocument() {
    return this.rawDocumentTransition;
  }
}

module.exports = MissingDocumentTransitionTypeError;
