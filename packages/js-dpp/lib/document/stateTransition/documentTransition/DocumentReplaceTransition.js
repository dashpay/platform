const AbstractDocumentTransition = require('./AbstractDocumentTransition');

class DocumentReplaceTransition extends AbstractDocumentTransition {
  /**
   * @param {RawDocumentReplaceTransition} rawTransition
   */
  constructor(rawTransition) {
    super(rawTransition);

    const data = { ...rawTransition };

    this.id = rawTransition.$id;
    this.type = rawTransition.$type;
    this.revision = rawTransition.$revision;

    if (rawTransition.$updatedAt) {
      this.updatedAt = new Date(rawTransition.$updatedAt);
    }

    delete data.$id;
    delete data.$type;
    delete data.$action;
    delete data.$revision;
    delete data.$dataContractId;
    delete data.$updatedAt;

    this.data = data;
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
   * Get id
   *
   * @returns {string}
   */
  getId() {
    return this.id;
  }

  /**
   * Get document type
   *
   * @return {string}
   */
  getType() {
    return this.type;
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
   * Get data
   *
   * @returns {Object}
   */
  getData() {
    return this.data;
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
   * @return {RawDocumentReplaceTransition}
   */
  toJSON() {
    const json = {
      ...super.toJSON(),
      $id: this.getId(),
      $type: this.getType(),
      $revision: this.getRevision(),
      ...this.data,
    };

    if (this.getUpdatedAt()) {
      json.$updatedAt = this.getUpdatedAt().getTime();
    }

    return json;
  }
}

/**
 * @typedef {Object} RawDocumentReplaceTransition
 * @property {number} $action
 * @property {string} $id
 * @property {string} $type
 * @property {number} $revision
 * @property {number} [$updatedAt]
 */

module.exports = DocumentReplaceTransition;
