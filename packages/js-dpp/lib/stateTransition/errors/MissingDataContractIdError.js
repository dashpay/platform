const DPPError = require('../../errors/DPPError');

class MissingDataContractIdError extends DPPError {
  /**
   * @param {
   *    RawDocumentCreateTransition|
   *    RawDocumentReplaceTransition|
   *    RawDocumentDeleteTransition
   * } rawDocumentTransition
   */
  constructor(rawDocumentTransition) {
    super('$dataContractId is not present');

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
    return this.rawDocument;
  }
}

module.exports = MissingDataContractIdError;
