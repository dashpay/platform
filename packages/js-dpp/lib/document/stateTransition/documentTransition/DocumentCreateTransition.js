const AbstractDocumentTransition = require('./AbstractDocumentTransition');

class DocumentCreateTransition extends AbstractDocumentTransition {
  /**
   * @param {RawDocumentCreateTransition} rawTransition
   */
  constructor(rawTransition) {
    super(rawTransition);

    const data = { ...rawTransition };

    this.id = rawTransition.$id;
    this.type = rawTransition.$type;
    this.entropy = rawTransition.$entropy;

    if (rawTransition.$createdAt) {
      this.createdAt = new Date(rawTransition.$createdAt);
    }

    if (rawTransition.$updatedAt) {
      this.updatedAt = new Date(rawTransition.$updatedAt);
    }

    delete data.$id;
    delete data.$type;
    delete data.$entropy;
    delete data.$action;
    delete data.$dataContractId;
    delete data.$createdAt;
    delete data.$updatedAt;

    this.data = data;
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
   * @returns {string}
   */
  getType() {
    return this.type;
  }

  /**
   * Get entropy
   *
   * @returns {string}
   */
  getEntropy() {
    return this.entropy;
  }

  /**
   * Get data
   *
   * @returns {Object}
   */
  getData() {
    return this.data;
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
   * Get document transition as a plain object
   *
   * @return {RawDocumentCreateTransition}
   */
  toJSON() {
    const json = {
      ...super.toJSON(),
      $id: this.getId(),
      $type: this.getType(),
      $entropy: this.getEntropy(),
      ...this.data,
    };

    if (this.getCreatedAt()) {
      json.$createdAt = this.getCreatedAt().getTime();
    }

    if (this.getUpdatedAt()) {
      json.$updatedAt = this.getUpdatedAt().getTime();
    }

    return json;
  }
}

/**
 * @typedef {Object} RawDocumentCreateTransition
 * @property {number} $action
 * @property {string} $id
 * @property {string} $type
 * @property {string} $entropy
 * @property {number} [$createdAt]
 * @property {number} [$updatedAt]
 */

DocumentCreateTransition.INITIAL_REVISION = 1;

module.exports = DocumentCreateTransition;
