const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const getBinaryPropertiesFromSchema = require('./getBinaryPropertiesFromSchema');

const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');
const Identifier = require('../identifier/Identifier');

class DataContract {
  /**
   * @param {RawDataContract} rawDataContract
   */
  constructor(rawDataContract) {
    this.protocolVersion = rawDataContract.protocolVersion;

    this.id = Identifier.from(rawDataContract.$id);
    this.ownerId = Identifier.from(rawDataContract.ownerId);

    this.setJsonMetaSchema(rawDataContract.$schema);
    this.setDocuments(rawDataContract.documents);
    this.setDefinitions(rawDataContract.$defs);

    this.binaryProperties = {};
  }

  /**
   * Get Data Contract protocol version
   *
   * @returns {number}
   */
  getProtocolVersion() {
    return this.protocolVersion;
  }

  /**
   * Get ID
   *
   * @return {Identifier}
   */
  getId() {
    return this.id;
  }

  /**
   * Get owner id
   *
   * @return {Identifier}
   */
  getOwnerId() {
    return this.ownerId;
  }

  /**
   * Get JSON Schema ID
   *
   * @return {string}
   */
  getJsonSchemaId() {
    return this.getId().toString();
  }

  /**
   *
   * @param {string} schema
   */
  setJsonMetaSchema(schema) {
    this.schema = schema;

    return this;
  }

  /**
   *
   * @return {string}
   */
  getJsonMetaSchema() {
    return this.schema;
  }

  /**
   *
   * @param {Object<string, Object>} documents
   * @return {DataContract}
   */
  setDocuments(documents) {
    this.documents = documents;

    return this;
  }

  /**
   *
   * @return {Object<string, Object>}
   */
  getDocuments() {
    return this.documents;
  }

  /**
   * Returns true if document type is defined
   *
   * @param {string} type
   * @return {boolean}
   */
  isDocumentDefined(type) {
    return Object.prototype.hasOwnProperty.call(this.documents, type);
  }

  /**
   *
   * @param {string} type
   * @param {object} schema
   * @return {DataContract}
   */
  setDocumentSchema(type, schema) {
    this.documents[type] = schema;

    return this;
  }

  /**
   *
   * @param {string} type
   * @return {Object}
   */
  getDocumentSchema(type) {
    if (!this.isDocumentDefined(type)) {
      throw new InvalidDocumentTypeError(type, this);
    }

    return this.documents[type];
  }

  /**
   * @param {string} type
   * @return {{$ref: string}}
   */
  getDocumentSchemaRef(type) {
    if (!this.isDocumentDefined(type)) {
      throw new InvalidDocumentTypeError(type, this);
    }

    return { $ref: `${this.getJsonSchemaId()}#/documents/${type}` };
  }

  /**
   * @param {Object<string, Object>} $defs
   * @return {DataContract}
   */
  setDefinitions($defs) {
    this.$defs = $defs;

    return this;
  }

  /**
   * @return {Object<string, Object>}
   */
  getDefinitions() {
    return this.$defs;
  }

  /**
   * Set Data Contract entropy
   *
   * @param {Buffer} entropy
   * @return {DataContract}
   */
  setEntropy(entropy) {
    this.entropy = entropy;

    return this;
  }

  /**
   * Get Data Contract entropy
   *
   * @return {Buffer}
   */
  getEntropy() {
    return this.entropy;
  }

  /**
   * Get properties with `contentEncoding` constraint
   *
   * @param {string} type
   *
   * @return {Object}
   */
  getBinaryProperties(type) {
    if (!this.isDocumentDefined(type)) {
      throw new InvalidDocumentTypeError(type, this);
    }

    if (this.binaryProperties[type]) {
      return this.binaryProperties[type];
    }

    this.binaryProperties[type] = getBinaryPropertiesFromSchema(
      this.documents[type],
    );

    return this.binaryProperties[type];
  }

  /**
   * Set metadata
   * @param {Metadata} metadata
   */
  setMetadata(metadata) {
    this.metadata = metadata;
  }

  /**
   * Get metadata
   * @returns {Metadata|null}
   */
  getMetadata() {
    return this.metadata;
  }

  /**
   * Return Data Contract as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   *
   * @return {RawDataContract}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        skipIdentifiersConversion: false,
        ...options,
      },
    );

    const rawDataContract = {
      protocolVersion: this.getProtocolVersion(),
      $id: this.getId(),
      $schema: this.getJsonMetaSchema(),
      ownerId: this.getOwnerId(),
      documents: this.getDocuments(),
    };

    if (!options.skipIdentifiersConversion) {
      rawDataContract.$id = this.getId().toBuffer();
      rawDataContract.ownerId = this.getOwnerId().toBuffer();
    }

    const $defs = this.getDefinitions();

    if ($defs && Object.getOwnPropertyNames($defs).length) {
      rawDataContract.$defs = $defs;
    }

    return rawDataContract;
  }

  /**
   * Return Data Contract as JSON object
   *
   * @return {JsonDataContract}
   */
  toJSON() {
    return {
      ...this.toObject({ skipIdentifiersConversion: true }),
      $id: this.getId().toString(),
      ownerId: this.getOwnerId().toString(),
    };
  }

  /**
   * Return Data Contract as a Buffer
   *
   * @returns {Buffer}
   */
  toBuffer() {
    const serializedData = this.toObject();
    delete serializedData.protocolVersion;

    const protocolVersionUInt32 = Buffer.alloc(4);
    protocolVersionUInt32.writeUInt32BE(this.getProtocolVersion(), 0);

    return Buffer.concat([protocolVersionUInt32, encode(serializedData)]);
  }

  /**
   * Returns hex string with Data Contract hash
   *
   * @return {Buffer}
   */
  hash() {
    return hash(this.toBuffer());
  }
}

/**
 * @typedef {Object} RawDataContract
 * @property {number} protocolVersion
 * @property {Buffer} $id
 * @property {string} $schema
 * @property {Buffer} ownerId
 * @property {Object<string, Object>} documents
 * @property {Object<string, Object>} [$defs]
 */

/**
 * @typedef {Object} JsonDataContract
 * @property {number} protocolVersion
 * @property {string} $id
 * @property {string} $schema
 * @property {string} ownerId
 * @property {Object<string, Object>} documents
 * @property {Object<string, Object>} [$defs]
 */

DataContract.DEFAULTS = {
  SCHEMA: 'https://schema.dash.org/dpp-0-4-0/meta/data-contract',
};

module.exports = DataContract;
