const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const cloneDeepRawData = require('../util/cloneDeepRawData');
const hash = require('../util/hash');
const { encode } = require('../util/serializer');
const transpileEncodedProperties = require('../util/encoding/transpileEncodedProperties');
const EncodedBuffer = require('../util/encoding/EncodedBuffer');

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
      this.id = EncodedBuffer.from(rawDocument.$id, EncodedBuffer.ENCODING.BASE58);
      delete data.$id;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$type')) {
      this.type = rawDocument.$type;
      delete data.$type;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$dataContractId')) {
      this.dataContractId = EncodedBuffer.from(
        rawDocument.$dataContractId,
        EncodedBuffer.ENCODING.BASE58,
      );
      delete data.$dataContractId;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$ownerId')) {
      this.ownerId = EncodedBuffer.from(rawDocument.$ownerId, EncodedBuffer.ENCODING.BASE58);
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
   * @return {EncodedBuffer}
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
   * @return {EncodedBuffer}
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
   * @return {EncodedBuffer}
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
   * @param {Buffer} entropy
   */
  setEntropy(entropy) {
    this.entropy = EncodedBuffer.from(entropy, EncodedBuffer.ENCODING.BASE58);
  }

  /**
   * Get entropy
   *
   * @return {EncodedBuffer}
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
    let encodedValue = cloneDeepRawData(value);

    const encodedProperties = this.dataContract.getBinaryProperties(
      this.getType(),
    );

    Object.entries(encodedProperties).forEach(([propertyPath]) => {
      if (path === propertyPath) {
        encodedValue = EncodedBuffer.from(value, EncodedBuffer.ENCODING.BASE64);
      } else if (propertyPath.includes(path)) {
        // in case of object we need to remove
        // first dot as we removed beginning of the path
        // e.g. `1.2.3` and '1.2` in the result would be `.2`
        const partialPath = propertyPath.substring(path.length + 1, propertyPath.length);
        const buffer = lodashGet(encodedValue, partialPath);

        if (buffer !== undefined) {
          lodashSet(
            encodedValue,
            partialPath,
            EncodedBuffer.from(buffer, EncodedBuffer.ENCODING.BASE64),
          );
        }
      }
    });

    lodashSet(this.data, path, encodedValue);

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
   * @return {JsonDocument}
   */
  toJSON() {
    const jsonDocument = {
      ...this.toObject({ encodedBuffer: true }),
      $id: this.getId().toString(),
      $dataContractId: this.getDataContractId().toString(),
      $ownerId: this.getOwnerId().toString(),
    };

    return transpileEncodedProperties(
      this.dataContract,
      this.getType(),
      jsonDocument,
      (encodedBuffer) => encodedBuffer.toString(),
    );
  }

  /**
   * Return Document as plain object (without converting encoded fields)
   *
   * @param {Object} [options]
   * @param {boolean} [options.encodedBuffer=false]
   * @return {RawDocument}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        encodedBuffer: false,
        ...options,
      },
    );

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

    if (!options.encodedBuffer) {
      rawDocument.$id = this.getId().toBuffer();
      rawDocument.$dataContractId = this.getDataContractId().toBuffer();
      rawDocument.$ownerId = this.getOwnerId().toBuffer();

      return transpileEncodedProperties(
        this.dataContract,
        this.getType(),
        rawDocument,
        (encodedBuffer) => encodedBuffer.toBuffer(),
      );
    }

    return rawDocument;
  }

  /**
   * Return serialized Document
   *
   * @return {Buffer}
   */
  toBuffer() {
    return encode(this.toObject());
  }

  /**
   * Returns hex string with object hash
   *
   * @return {Buffer}
   */
  hash() {
    return hash(this.toBuffer());
  }

  /**
   * Create document from JSON
   *
   * @param {JsonDocument} jsonDocument
   * @param {DataContract} dataContract
   *
   * @return {Document}
   */
  static fromJSON(jsonDocument, dataContract) {
    const rawDocument = transpileEncodedProperties(
      dataContract,
      jsonDocument.$type,
      jsonDocument,
      (string, encoding) => EncodedBuffer.from(string, encoding).toBuffer(),
    );

    return new Document(rawDocument, dataContract);
  }
}

/**
 * @typedef {Object} RawDocument
 * @property {number} $protocolVersion
 * @property {Buffer} $id
 * @property {string} $type
 * @property {Buffer} $dataContractId
 * @property {Buffer} $ownerId
 * @property {number} $revision
 * @property {number} [$createdAt]
 * @property {number} [$updatedAt]
 */

/**
 * @typedef {Object} JsonDocument
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
