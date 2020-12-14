const AbstractDocumentTransition = require('./AbstractDocumentTransition');
const AbstractDataDocumentTransition = require('./AbstractDataDocumentTransition');

class DocumentReplaceTransition extends AbstractDataDocumentTransition {
  /**
   * @param {RawDocumentReplaceTransition} rawTransition
   * @param {DataContract} dataContract
   */
  constructor(rawTransition, dataContract) {
    super(rawTransition, dataContract);

    if (Object.prototype.hasOwnProperty.call(rawTransition, '$revision')) {
      this.revision = rawTransition.$revision;
    }

    if (Object.prototype.hasOwnProperty.call(rawTransition, '$updatedAt')) {
      this.updatedAt = new Date(rawTransition.$updatedAt);
    }
  }

  /**
   * Get action
   *
   * @returns {number}
   */
  getAction() {
    return AbstractDocumentTransition.ACTIONS.REPLACE;
  }

  /**
   * Get next document revision
   *
   * @return {number}
   */
  getRevision() {
    return this.revision;
  }

  /**
   * Get update date
   *
   * @return {Date}
   */
  getUpdatedAt() {
    return this.updatedAt;
  }

  /**
   * Get plain object representation
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   * @return {RawDocumentReplaceTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        skipIdentifiersConversion: false,
        ...options,
      },
    );

    const rawDocumentTransition = {
      ...super.toObject(options),
      $revision: this.getRevision(),
    };

    if (this.getUpdatedAt()) {
      rawDocumentTransition.$updatedAt = this.getUpdatedAt().getTime();
    }

    return rawDocumentTransition;
  }
}

/**
 * @typedef {RawDocumentTransition & Object} RawDocumentReplaceTransition
 * @property {number} $revision
 * @property {number} [$updatedAt]
 */

/**
 * @typedef {JsonDocumentTransition & Object} JsonDocumentReplaceTransition
 * @property {number} $revision
 * @property {number} [$updatedAt]
 */

module.exports = DocumentReplaceTransition;
