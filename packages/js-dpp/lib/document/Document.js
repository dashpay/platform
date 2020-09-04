const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const hash = require('../util/hash');
const { encode } = require('../util/serializer');
const encodeToBase64WithoutPadding = require('../util/encodeToBase64WithoutPadding');

class Document {
  /**
   * @param {RawDocument} rawDocument
   * @param {DataContract} dataContract
   */
  constructor(rawDocument, dataContract) {
    this.dataContract = dataContract;

    const data = { ...rawDocument };

    this.entropy = undefined;

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$protocolVersion')) {
      this.protocolVersion = rawDocument.$protocolVersion;
      delete data.$protocolVersion;
    }

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

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$createdAt')) {
      this.createdAt = new Date(rawDocument.$createdAt);
      delete data.$createdAt;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$updatedAt')) {
      this.updatedAt = new Date(rawDocument.$updatedAt);
      delete data.$updatedAt;
    }

    this.setData(data);
  }

  /**
   * Get Document protocol version
   *
   * @returns {number}
   */
  getProtocolVersion() {
    return this.protocolVersion;
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
   * Get Data Contract
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this.dataContract;
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
   * Set document creation date
   *
   * @param {Date} date
   * @return {Document}
   */
  setCreatedAt(date) {
    this.createdAt = date;

    return this;
  }

  /**
   * Get document creation date
   *
   * @return {Date}
   */
  getCreatedAt() {
    return this.createdAt;
  }

  /**
   * Set document updated date
   *
   * @param {Date} date
   * @return {Document}
   */
  setUpdatedAt(date) {
    this.updatedAt = date;

    return this;
  }

  /**
   * Get document updated date
   *
   * @return {Date}
   */
  getUpdatedAt() {
    return this.updatedAt;
  }

  /**
   * Return Document as JSON object
   *
   * @return {RawDocument}
   */
  toJSON() {
    const rawDocument = this.toObject();

    const encodedProperties = this.dataContract.getEncodedProperties(
      this.getType(),
    );

    Object.keys(encodedProperties)
      .forEach((propertyPath) => {
        const property = encodedProperties[propertyPath];

        if (property.contentEncoding === 'base64') {
          const value = lodashGet(rawDocument, propertyPath);
          if (value !== undefined) {
            lodashSet(
              rawDocument,
              propertyPath,
              encodeToBase64WithoutPadding(
                Buffer.from(value),
              ),
            );
          }
        }
      });

    return rawDocument;
  }

  /**
   * Return Document as plain object (without converting encoded fields)
   *
   * @return {RawDocument}
   */
  toObject() {
    const rawDocument = {
      $protocolVersion: this.getProtocolVersion(),
      $id: this.getId(),
      $type: this.getType(),
      $dataContractId: this.getDataContractId(),
      $ownerId: this.getOwnerId(),
      $revision: this.getRevision(),
      ...this.getData(),
    };

    if (this.createdAt) {
      rawDocument.$createdAt = this.getCreatedAt().getTime();
    }

    if (this.updatedAt) {
      rawDocument.$updatedAt = this.getUpdatedAt().getTime();
    }

    return rawDocument;
  }

  /**
   * Return serialized Document
   *
   * @return {Buffer}
   */
  serialize() {
    const plainObject = this.toObject();

    return encode(plainObject);
  }

  /**
   * Returns hex string with object hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize()).toString('hex');
  }

  /**
   * Create document from JSON
   *
   * @param {RawDocument} rawDocument
   * @param {DataContract} dataContract
   *
   * @return {Document}
   */
  static fromJSON(rawDocument, dataContract) {
    const encodedProperties = dataContract.getEncodedProperties(
      rawDocument.$type,
    );

    Object.keys(encodedProperties)
      .forEach((propertyPath) => {
        const property = encodedProperties[propertyPath];

        if (property.contentEncoding === 'base64') {
          const value = lodashGet(rawDocument, propertyPath);
          if (value !== undefined) {
            lodashSet(
              rawDocument,
              propertyPath,
              Buffer.from(value, 'base64'),
            );
          }
        }
      });

    return new Document(rawDocument, dataContract);
  }
}

/**
 * @typedef {Object} RawDocument
 * @property {number} $protocolVersion
 * @property {string} $id
 * @property {string} $type
 * @property {string} $dataContractId
 * @property {string} $ownerId
 * @property {number} $revision
 * @property {number} [$createdAt]
 * @property {number} [$updatedAt]
 */

Document.PROTOCOL_VERSION = 0;

Document.SYSTEM_PREFIX = '$';

module.exports = Document;
