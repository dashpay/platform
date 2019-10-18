const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');

class DataContract {
  /**
   * @param {string} contractId
   * @param {Object<string, Object>} documents
   */
  constructor(contractId, documents) {
    this.contractId = contractId;

    this.setVersion(DataContract.DEFAULTS.VERSION);
    this.setJsonMetaSchema(DataContract.DEFAULTS.SCHEMA);
    this.setDocuments(documents);
    this.setDefinitions({});
  }

  /**
   * Get ID
   *
   * @return {string}
   */
  getId() {
    return this.contractId;
  }

  /**
   * Get JSON Schema ID
   *
   * @return {string}
   */
  getJsonSchemaId() {
    return DataContract.SCHEMA_ID;
  }

  /**
   * Set version
   *
   * @param {number} version
   * @return {DataContract}
   */
  setVersion(version) {
    this.version = version;

    return this;
  }

  /**
   * Get version
   *
   * @return {number}
   */
  getVersion() {
    return this.version;
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
   * Return Data Contract as plain object
   *
   * @return {RawDataContract}
   */
  toJSON() {
    const json = {
      $schema: this.getJsonMetaSchema(),
      contractId: this.getId(),
      version: this.getVersion(),
      documents: this.getDocuments(),
    };

    const definitions = this.getDefinitions();

    if (Object.getOwnPropertyNames(definitions).length) {
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
}

DataContract.DEFAULTS = {
  VERSION: 1,
  SCHEMA: 'https://schema.dash.org/dpp-0-4-0/meta/data-contract',
};

DataContract.SCHEMA_ID = 'dataContract';

module.exports = DataContract;
