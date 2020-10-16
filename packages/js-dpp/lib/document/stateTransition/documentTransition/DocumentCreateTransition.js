const AbstractDocumentTransition = require('./AbstractDocumentTransition');
const AbstractDataDocumentTransition = require('./AbstractDataDocumentTransition');

class DocumentCreateTransition extends AbstractDataDocumentTransition {
  /**
   * @param {RawDocumentCreateTransition} rawTransition
   * @param {DataContract} dataContract
   */
  constructor(rawTransition, dataContract) {
    super(rawTransition, dataContract);

    if (Object.prototype.hasOwnProperty.call(rawTransition, '$entropy')) {
      this.entropy = rawTransition.$entropy;
    }

    if (Object.prototype.hasOwnProperty.call(rawTransition, '$createdAt')) {
      this.createdAt = new Date(rawTransition.$createdAt);
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
    return AbstractDocumentTransition.ACTIONS.CREATE;
  }

  /**
   * Get entropy
   *
   * @returns {Buffer}
   */
  getEntropy() {
    return this.entropy;
  }

  /**
   * Get creation date
   *
   * @return {Date}
   */
  getCreatedAt() {
    return this.createdAt;
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
   * @return {RawDocumentCreateTransition}
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
      $entropy: this.getEntropy(),
    };

    if (this.createdAt) {
      rawDocumentTransition.$createdAt = this.getCreatedAt().getTime();
    }

    if (this.updatedAt) {
      rawDocumentTransition.$updatedAt = this.getUpdatedAt().getTime();
    }

    return rawDocumentTransition;
  }
}

/**
 * @typedef {RawDocumentTransition & Object} RawDocumentCreateTransition
 * @property {Buffer} $entropy
 * @property {number} [$createdAt]
 * @property {number} [$updatedAt]
 */

/**
 * @typedef {JsonDocumentTransition & Object} JsonDocumentCreateTransition
 * @property {string} $entropy
 * @property {number} [$createdAt]
 * @property {number} [$updatedAt]
 */

DocumentCreateTransition.INITIAL_REVISION = 1;

module.exports = DocumentCreateTransition;
