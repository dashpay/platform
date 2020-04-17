const AbstractStateTransitionIdentitySigned = require('../../stateTransition/AbstractStateTransitionIdentitySigned');
const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

const AbstractDocumentTransition = require('./documentTransition/AbstractDocumentTransition');
const DocumentCreateTransition = require('./documentTransition/DocumentCreateTransition');
const DocumentReplaceTransition = require('./documentTransition/DocumentReplaceTransition');
const DocumentDeleteTransition = require('./documentTransition/DocumentDeleteTransition');

const actionsToClasses = {
  [AbstractDocumentTransition.ACTIONS.CREATE]: DocumentCreateTransition,
  [AbstractDocumentTransition.ACTIONS.REPLACE]: DocumentReplaceTransition,
  [AbstractDocumentTransition.ACTIONS.DELETE]: DocumentDeleteTransition,
};

class DocumentsBatchTransition extends AbstractStateTransitionIdentitySigned {
  /**
   * @param {RawDocumentsBatchTransition} [rawStateTransition]
   */
  constructor(rawStateTransition = {}) {
    super(rawStateTransition);

    this.ownerId = rawStateTransition.ownerId;

    this.transitions = (rawStateTransition.transitions || []).map((rawDocumentTransition) => (
      new actionsToClasses[rawDocumentTransition.$action](rawDocumentTransition)
    ));
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return stateTransitionTypes.DOCUMENTS_BATCH;
  }

  /**
   * Get owner id
   *
   * @return {string}
   */
  getOwnerId() {
    return this.ownerId;
  }

  /**
   * Get document action transitions
   *
   * @return {DocumentCreateTransition[]|DocumentReplaceTransition[]|DocumentDeleteTransition[]}
   */
  getTransitions() {
    return this.transitions;
  }

  /**
   * Get Documents State Transition as plain object
   *
   * @param {Object} [options]
   * @return {RawDocumentsBatchTransition}
   */
  toJSON(options = {}) {
    return {
      ...super.toJSON(options),
      ownerId: this.getOwnerId(),
      transitions: this.getTransitions().map((t) => t.toJSON()),
    };
  }
}

/**
 * @typedef {Object} RawDocumentsBatchTransition
 * @property {number} protocolVersion
 * @property {number} type
 * @property {string} ownerId
 * @property {
 *   Array.<RawDocumentCreateTransition|RawDocumentReplaceTransition|RawDocumentDeleteTransition>
 * } transitions
 * @property {number|null} signaturePublicKeyId
 * @property {string|null} signature
 */

module.exports = DocumentsBatchTransition;
