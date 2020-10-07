const AbstractDocumentTransition = require('./AbstractDocumentTransition');

const transpileEncodedProperties = require('../../../util/encoding/transpileEncodedProperties');
const EncodedBuffer = require('../../../util/encoding/EncodedBuffer');

class DocumentCreateTransition extends AbstractDocumentTransition {
  /**
   * @param {RawDocumentCreateTransition} rawTransition
   * @param {DataContract} dataContract
   */
  constructor(rawTransition, dataContract) {
    super(rawTransition, dataContract);

    const data = { ...rawTransition };

    if (Object.prototype.hasOwnProperty.call(rawTransition, '$entropy')) {
      this.entropy = EncodedBuffer.from(rawTransition.$entropy, EncodedBuffer.ENCODING.BASE58);
    }

    if (Object.prototype.hasOwnProperty.call(rawTransition, '$createdAt')) {
      this.createdAt = new Date(rawTransition.$createdAt);
    }

    if (Object.prototype.hasOwnProperty.call(rawTransition, '$updatedAt')) {
      this.updatedAt = new Date(rawTransition.$updatedAt);
    }

    delete data.$id;
    delete data.$type;
    delete data.$entropy;
    delete data.$action;
    delete data.$dataContractId;
    delete data.$createdAt;
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
    return AbstractDocumentTransition.ACTIONS.CREATE;
  }

  /**
   * Get entropy
   *
   * @returns {EncodedBuffer}
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
   * Get plain object representation
   *
   * @param {Object} [options]
   * @param {boolean} [options.encodedBuffer=false]
   * @return {RawDocumentCreateTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        encodedBuffer: false,
        ...options,
      },
    );

    let rawDocumentTransition = {
      ...super.toObject(options),
      $entropy: this.getEntropy(),
      ...this.getData(),
    };

    if (this.createdAt) {
      rawDocumentTransition.$createdAt = this.getCreatedAt().getTime();
    }

    if (this.updatedAt) {
      rawDocumentTransition.$updatedAt = this.getUpdatedAt().getTime();
    }

    if (!options.encodedBuffer) {
      rawDocumentTransition = {
        ...rawDocumentTransition,
        $id: this.getId().toBuffer(),
        $entropy: this.getEntropy().toBuffer(),
      };

      return transpileEncodedProperties(
        this.dataContract,
        this.getType(),
        rawDocumentTransition,
        (encodedBuffer) => encodedBuffer.toBuffer(),
      );
    }

    return rawDocumentTransition;
  }

  /**
   * Get JSON representation
   *
   * @return {JsonDocumentCreateTransition}
   */
  toJSON() {
    const jsonDocumentTransition = {
      ...super.toJSON(),
      $entropy: this.getEntropy().toString(),
    };

    return transpileEncodedProperties(
      this.dataContract,
      this.getType(),
      jsonDocumentTransition,
      (encodedBuffer) => encodedBuffer.toString(),
    );
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
