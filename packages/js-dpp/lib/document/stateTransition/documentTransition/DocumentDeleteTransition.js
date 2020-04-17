const AbstractDocumentTransition = require('./AbstractDocumentTransition');

class DocumentDeleteTransition extends AbstractDocumentTransition {
  /**
   * @param {RawDocumentDeleteTransition} rawTransition
   */
  constructor(rawTransition) {
    super(rawTransition);

    this.id = rawTransition.$id;
    this.type = rawTransition.$type;
  }

  /**
   * Get action
   *
   * @returns {number}
   */
  getAction() {
    return AbstractDocumentTransition.ACTIONS.DELETE;
  }

  /**
   * Get id
   *
   * @returns {string}
   */
  getId() {
    return this.id;
  }

  /**
   * Get type
   *
   * @returns {*}
   */
  getType() {
    return this.type;
  }

  /**
   * Get document transition as a plain object
   *
   * @return {RawDocumentDeleteTransition}
   */
  toJSON() {
    return {
      ...super.toJSON(),
      $id: this.getId(),
      $type: this.getType(),
    };
  }
}

/**
 * @typedef {Object} RawDocumentDeleteTransition
 * @property {number} $action
 * @property {string} $id
 * @property {string} $type
 */

module.exports = DocumentDeleteTransition;
