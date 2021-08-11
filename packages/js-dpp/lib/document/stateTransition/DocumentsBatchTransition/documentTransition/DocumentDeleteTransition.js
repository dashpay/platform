const AbstractDocumentTransition = require('./AbstractDocumentTransition');

class DocumentDeleteTransition extends AbstractDocumentTransition {
  /**
   * Get action
   *
   * @returns {number}
   */
  getAction() {
    return AbstractDocumentTransition.ACTIONS.DELETE;
  }
}

/**
 * @typedef {RawDocumentTransition & Object} RawDocumentDeleteTransition
 */

/**
 * @typedef {JsonDocumentTransition & Object} JsonDocumentDeleteTransition
 */

module.exports = DocumentDeleteTransition;
