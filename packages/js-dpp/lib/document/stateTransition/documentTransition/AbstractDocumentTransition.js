const lodashGet = require('lodash.get');

const Identifier = require('../../../identifier/Identifier');

/**
 * @abstract
 */
class AbstractDocumentTransition {
  constructor(rawDocumentTransition, dataContract) {
    this.type = rawDocumentTransition.$type;

    if (Object.prototype.hasOwnProperty.call(rawDocumentTransition, '$id')) {
      this.id = Identifier.from(rawDocumentTransition.$id);
    }

    if (Object.prototype.hasOwnProperty.call(rawDocumentTransition, '$dataContractId')) {
      this.dataContractId = Identifier.from(rawDocumentTransition.$dataContractId);
    }

    this.dataContract = dataContract;
  }

  /**
   * Get id
   *
   * @returns {Identifier}
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
   * @return {Identifier}
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
   * @param {boolean} [options.skipIdentifiersConversion=false]
   * @return {RawDocumentTransition}
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
      $id: this.getId(),
      $type: this.getType(),
      $action: this.getAction(),
      $dataContractId: this.getDataContractId(),
    };

    if (!options.skipIdentifiersConversion) {
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
    return {
      ...this.toObject({ skipIdentifiersConversion: true }),
      $id: this.getId().toString(),
      $dataContractId: this.getDataContractId().toString(),
    };
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
