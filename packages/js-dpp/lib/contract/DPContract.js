const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');

class DPContract {
  /**
   * @param {string} name
   * @param {Object<string, Object>} documents
   */
  constructor(name, documents) {
    this.setName(name);
    this.setVersion(DPContract.DEFAULTS.VERSION);
    this.setJsonMetaSchema(DPContract.DEFAULTS.SCHEMA);
    this.setDocuments(documents);
    this.setDefinitions({});
  }

  /**
   * Calculate DP Contract ID
   *
   * @return {string}
   */
  getId() {
    // TODO: Id should be unique for whole network
    //  so we need something like BUID for DP Contracts or use ST hash what is not so flexible
    return this.hash();
  }

  /**
   * Get JSON Schema ID
   *
   * @return {string}
   */
  getJsonSchemaId() {
    return DPContract.SCHEMA_ID;
  }

  /**
   *
   * @param {string} name
   * @return {DPContract}
   */
  setName(name) {
    this.name = name;

    return this;
  }

  /**
   *
   * @return {string}
   */
  getName() {
    return this.name;
  }

  /**
   *
   * @param {number} version
   * @return {DPContract}
   */
  setVersion(version) {
    this.version = version;

    return this;
  }

  /**
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
   * @return {DPContract}
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
   * @return {DPContract}
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
   * @return {DPContract}
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
   * Return DP Contract as plain object
   *
   * @return {{$schema: string,
   *           name: string,
   *           version: number,
   *           documents: Object<string, Object>,
   *           [definitions]: Object<string, Object>}}
   */
  toJSON() {
    const json = {
      $schema: this.getJsonMetaSchema(),
      name: this.getName(),
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
   * Return serialized DP Contract
   *
   * @return {Buffer}
   */
  serialize() {
    return encode(this.toJSON());
  }

  /**
   * Returns hex string with contract hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize());
  }
}

DPContract.DEFAULTS = {
  VERSION: 1,
  SCHEMA: 'https://schema.dash.org/dpp-0-4-0/meta/dp-contract',
};

DPContract.SCHEMA_ID = 'dp-contract';

module.exports = DPContract;
