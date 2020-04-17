const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const hash = require('../util/hash');
const { encode } = require('../util/serializer');

class Document {
  /**
   * @param {RawDocument} rawDocument
   */
  constructor(rawDocument) {
    const data = { ...rawDocument };

    this.entropy = undefined;

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$id')) {
      this.id = rawDocument.$id;
      delete data.$id;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$type')) {
      this.type = rawDocument.$type;
      delete data.$type;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$dataContractId')) {
      this.dataContractId = rawDocument.$dataContractId;
      delete data.$dataContractId;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$ownerId')) {
      this.ownerId = rawDocument.$ownerId;
      delete data.$ownerId;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$revision')) {
      this.revision = rawDocument.$revision;
      delete data.$revision;
    }

    this.setData(data);
  }

  /**
   * Get ID
   *
   * @return {string}
   */
  getId() {
    return this.id;
  }

  /**
   * Get type
   *
   * @return {string}
   */
  getType() {
    return this.type;
  }

  /**
   * Get Data Contract ID
   *
   * @return {string}
   */
  getDataContractId() {
    return this.dataContractId;
  }

  /**
   * Get Owner ID
   *
   * @return {string}
   */
  getOwnerId() {
    return this.ownerId;
  }

  /**
   * Set revision
   *
   * @param {number} revision
   * @return Document
   */
  setRevision(revision) {
    this.revision = revision;

    return this;
  }

  /**
   * Get revision
   *
   * @return {number}
   */
  getRevision() {
    return this.revision;
  }

  /**
   * Set entropy
   *
   * @param {string} entropy
   */
  setEntropy(entropy) {
    this.entropy = entropy;
  }

  /**
   * Get entropy
   *
   * @return {string}
   */
  getEntropy() {
    return this.entropy;
  }

  /**
   * Set data
   *
   * @param {Object} data
   * @return {Document}
   */
  setData(data) {
    this.data = {};

    Object.entries(data)
      .forEach(([name, value]) => this.set(name, value));

    return this;
  }

  /**
   * Get data
   *
   * @return {Object}
   */
  getData() {
    return this.data;
  }

  /**
   * Retrieves the field specified by {path}
   *
   * @param {string} path
   * @return {*}
   */
  get(path) {
    return lodashGet(this.data, path);
  }

  /**
   * Set the field specified by {path}
   *
   * @param {string} path
   * @param {*} value
   * @return {Document}
   */
  set(path, value) {
    lodashSet(this.data, path, value);

    return this;
  }

  /**
   * Return Document as plain object
   *
   * @return {RawDocument}
   */
  toJSON() {
    return {
      $id: this.getId(),
      $type: this.getType(),
      $dataContractId: this.getDataContractId(),
      $ownerId: this.getOwnerId(),
      $revision: this.getRevision(),
      ...this.getData(),
    };
  }

  /**
   * Return serialized Document
   *
   * @return {Buffer}
   */
  serialize() {
    const json = this.toJSON();

    return encode(json);
  }

  /**
   * Returns hex string with object hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize()).toString('hex');
  }
}

/**
 * @typedef {Object} RawDocument
 * @property {string} $id
 * @property {string} $type
 * @property {string} $dataContractId
 * @property {string} $ownerId
 * @property {number} $revision
 */

Document.SYSTEM_PREFIX = '$';

module.exports = Document;
