const AbstractStateError = require('../../AbstractStateError');

/**
 * @abstract
 */
class AbstractDataTriggerError extends AbstractStateError {
  /**
   * @param {Buffer} dataContractId
   * @param {Buffer} documentTransitionId
   * @param {string} message
   */
  constructor(dataContractId, documentTransitionId, message) {
    super(message);

    this.dataContractId = dataContractId;
    this.documentTransitionId = documentTransitionId;
  }

  /**
   * @returns {Buffer}
   */
  getDocumentTransitionId() {
    return this.documentTransitionId;
  }

  /**
   * @returns {Buffer}
   */
  getDataContractId() {
    return this.dataContractId;
  }

  /**
   * @param {Identifier} ownerId
   */
  setOwnerId(ownerId) {
    this.ownerId = ownerId;
  }

  /**
   * Get data trigger owner id
   *
   * @return {Identifier}
   */
  getOwnerId() {
    return this.ownerId;
  }

  /**
   * @param {
   *   DocumentCreateTransition|DocumentReplaceTransition|DocumentDeleteTransition
   * } documentTransition
   */
  setDocumentTransition(documentTransition) {
    this.documentTransition = documentTransition;
  }

  /**
   * Get document transition
   *
   * @returns {
   *   DocumentCreateTransition|DocumentReplaceTransition|DocumentDeleteTransition
   * }
   */
  getDocumentTransition() {
    return this.documentTransition;
  }
}

module.exports = AbstractDataTriggerError;
