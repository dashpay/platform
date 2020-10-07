const AbstractDocumentTransition = require('./AbstractDocumentTransition');

const transpileEncodedProperties = require('../../../util/encoding/transpileEncodedProperties');
const EncodedBuffer = require('../../../util/encoding/EncodedBuffer');

class DocumentReplaceTransition extends AbstractDocumentTransition {
  /**
   * @param {RawDocumentReplaceTransition} rawTransition
   * @param {DataContract} dataContract
   */
  constructor(rawTransition, dataContract) {
    super(rawTransition, dataContract);

    const data = { ...rawTransition };

    this.revision = rawTransition.$revision;

    if (Object.prototype.hasOwnProperty.call(rawTransition, '$updatedAt')) {
      this.updatedAt = new Date(rawTransition.$updatedAt);
    }

    delete data.$id;
    delete data.$type;
    delete data.$action;
    delete data.$revision;
    delete data.$dataContractId;
    delete data.$updatedAt;

    this.data = transpileEncodedProperties(
      dataContract,
      this.getType(),
      data,
      (buffer, encoding) => new EncodedBuffer(buffer, encoding),
    );
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
   * @returns {EncodedBuffer}
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
   * Get JSON representation
   *
   * @return {JsonDocumentReplaceTransition}
   */
  toJSON() {
    const jsonDocumentTransition = super.toJSON();

    return transpileEncodedProperties(
      this.dataContract,
      this.getType(),
      jsonDocumentTransition,
      (encodedBuffer) => encodedBuffer.toString(),
    );
  }

  /**
   * Get plain object representation
   *
   * @param {Object} [options]
   * @param {boolean} [options.encodedBuffer=false]
   * @return {RawDocumentReplaceTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        encodedBuffer: false,
        ...options,
      },
    );

    const rawDocumentTransition = {
      ...super.toObject(options),
      $revision: this.getRevision(),
      ...this.getData(),
    };

    if (this.getUpdatedAt()) {
      rawDocumentTransition.$updatedAt = this.getUpdatedAt().getTime();
    }

    if (!options.encodedBuffer) {
      return transpileEncodedProperties(
        this.dataContract,
        this.getType(),
        rawDocumentTransition,
        (encodedBuffer) => encodedBuffer.toBuffer(),
      );
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
