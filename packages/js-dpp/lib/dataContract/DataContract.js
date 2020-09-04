const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const getEncodedPropertiesFromSchema = require('./getEncodedPropertiesFromSchema');

const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');

class DataContract {
  /**
   * @param {RawDataContract} rawDataContract
   */
  constructor(rawDataContract) {
    this.id = rawDataContract.$id;
    this.ownerId = rawDataContract.ownerId;
    this.protocolVersion = rawDataContract.protocolVersion;

    this.setJsonMetaSchema(rawDataContract.$schema);
    this.setDocuments(rawDataContract.documents);
    this.setDefinitions(rawDataContract.definitions);

    this.encodedProperties = {};
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
   * @return {string}
   */
  getId() {
    return this.id;
  }

  /**
   * Get owner id
   *
   * @return {string}
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
    return this.getId();
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
   * @param {Object<string, Object>} definitions
   * @return {DataContract}
   */
  setDefinitions(definitions) {
    this.definitions = definitions;

    return this;
  }

  /**
   * @return {Object<string, Object>}
   */
  getDefinitions() {
    return this.definitions;
  }

  /**
   * Get properties with `contentEncoding` constraint
   *
   * @param {string} type
   *
   * @return {Object}
   */
  getEncodedProperties(type) {
    if (!this.isDocumentDefined(type)) {
      throw new InvalidDocumentTypeError(type, this);
    }

    if (this.encodedProperties[type]) {
      return this.encodedProperties[type];
    }

    this.encodedProperties[type] = getEncodedPropertiesFromSchema(
      this.documents[type],
    );

    return this.encodedProperties[type];
  }

  /**
   * Return Data Contract as plain object
   *
   * @return {RawDataContract}
   */
  toJSON() {
    const json = {
      protocolVersion: this.getProtocolVersion(),
      $id: this.getId(),
      $schema: this.getJsonMetaSchema(),
      ownerId: this.getOwnerId(),
      documents: this.getDocuments(),
    };

    const definitions = this.getDefinitions();

    if (definitions && Object.getOwnPropertyNames(definitions).length) {
      json.definitions = definitions;
    }

    return json;
  }

  /**
   * Return serialized Data Contract
   *
   * @return {Buffer}
   */
  serialize() {
    return encode(this.toJSON());
  }

  /**
   * Returns hex string with Data Contract hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize()).toString('hex');
  }

  /**
   * Set Data Contract entropy
   *
   * @param {string} entropy
   * @return {DataContract}
   */
  setEntropy(entropy) {
    this.entropy = entropy;

    return this;
  }

  /**
   * Get Data Contract entropy
   *
   * @return {string}
   */
  getEntropy() {
    return this.entropy;
  }
}

/**
 * @typedef {Object} RawDataContract
 * @property {number} protocolVersion
 * @property {string} $id
 * @property {string} $schema
 * @property {string} ownerId
 * @property {Object<string, Object>} documents
 * @property {Object<string, Object>} [definitions]
 */

DataContract.PROTOCOL_VERSION = 0;

DataContract.DEFAULTS = {
  SCHEMA: 'https://schema.dash.org/dpp-0-4-0/meta/data-contract',
};

module.exports = DataContract;
