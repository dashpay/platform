const lodashGet = require('lodash.get');

const transpileEncodedProperties = require('../../../util/encoding/transpileEncodedProperties');
const EncodedBuffer = require('../../../util/encoding/EncodedBuffer');

/**
 * @abstract
 */
class AbstractDocumentTransition {
  constructor(rawDocumentTransition, dataContract) {
    this.type = rawDocumentTransition.$type;

    if (Object.prototype.hasOwnProperty.call(rawDocumentTransition, '$id')) {
      this.id = EncodedBuffer.from(rawDocumentTransition.$id, EncodedBuffer.ENCODING.BASE58);
    }

    if (Object.prototype.hasOwnProperty.call(rawDocumentTransition, '$dataContractId')) {
      this.dataContractId = EncodedBuffer.from(
        rawDocumentTransition.$dataContractId,
        EncodedBuffer.ENCODING.BASE58,
      );
    }

    this.dataContract = dataContract;
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
   * Get type
   *
   * @returns {*}
   */
  getType() {
    return this.type;
  }

  /**
   * @abstract
   */
  getAction() {
    throw new Error('Not implemented');
  }

  /**
   * Get Data Contract ID
   *
   * @return {EncodedBuffer}
   */
  getDataContractId() {
    return this.dataContractId;
  }

  /**
   * Get transition property by path
   *
   * @param {string} propertyPath
   *
   * @return {*}
   */
  get(propertyPath) {
    return lodashGet(this.getData(), propertyPath);
  }

  /**
   * Get plain object representation
   *
   * @param {Object} [options]
   * @param {boolean} [options.encodedBuffer=false]
   * @return {RawDocumentTransition}
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
      $id: this.getId(),
      $type: this.getType(),
      $action: this.getAction(),
      $dataContractId: this.getDataContractId(),
    };

    if (!options.encodedBuffer) {
      rawDocumentTransition.$id = this.getId().toBuffer();
      rawDocumentTransition.$dataContractId = this.getDataContractId().toBuffer();
    }

    return rawDocumentTransition;
  }

  /**
   * Get JSON representation
   *
   * @return {JsonDocumentTransition}
   */
  toJSON() {
    const jsonDocumentTransition = {
      ...this.toObject({ encodedBuffer: true }),
      $id: this.getId().toString(),
      $dataContractId: this.getDataContractId().toString(),
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
 * @typedef {Object} RawDocumentTransition
 * @property {Buffer} $id
 * @property {string} $type
 * @property {number} $action
 * @property {Buffer} $dataContractId
 */

/**
 * @typedef {Object} JsonDocumentTransition
 * @property {string} $id
 * @property {string} $type
 * @property {number} $action
 * @property {string} $dataContractId
 */

AbstractDocumentTransition.ACTIONS = {
  CREATE: 0,
  REPLACE: 1,
  // 2 reserved for UPDATE
  DELETE: 3,
};

AbstractDocumentTransition.ACTION_NAMES = {
  CREATE: 'create',
  REPLACE: 'replace',
  DELETE: 'delete',
};

module.exports = AbstractDocumentTransition;
