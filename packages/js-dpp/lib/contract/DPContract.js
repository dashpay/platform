const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const InvalidDPObjectTypeError = require('../errors/InvalidDPObjectTypeError');

class DPContract {
  /**
   * @param {string} name
   * @param {Object<string, Object>} dpObjectsDefinition
   */
  constructor(name, dpObjectsDefinition) {
    this.setName(name);
    this.setVersion(DPContract.DEFAULTS.VERSION);
    this.setJsonMetaSchema(DPContract.DEFAULTS.SCHEMA);
    this.setDPObjectsDefinition(dpObjectsDefinition);
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
   * @param {Object<string, Object>} dpObjectsDefinition
   * @return {DPContract}
   */
  setDPObjectsDefinition(dpObjectsDefinition) {
    this.dpObjectsDefinition = dpObjectsDefinition;

    return this;
  }

  /**
   *
   * @return {Object<string, Object>}
   */
  getDPObjectsDefinition() {
    return this.dpObjectsDefinition;
  }

  /**
   * Returns true if object type is defined
   *
   * @param {string} type
   * @return {boolean}
   */
  isDPObjectDefined(type) {
    return Object.prototype.hasOwnProperty.call(this.dpObjectsDefinition, type);
  }

  /**
   *
   * @param {string} type
   * @param {object} schema
   * @return {DPContract}
   */
  setDPObjectSchema(type, schema) {
    this.dpObjectsDefinition[type] = schema;

    return this;
  }

  /**
   *
   * @param {string} type
   * @return {Object}
   */
  getDPObjectSchema(type) {
    if (!this.isDPObjectDefined(type)) {
      throw new InvalidDPObjectTypeError(type, this);
    }

    return this.dpObjectsDefinition[type];
  }

  /**
   * @param {string} type
   * @return {{$ref: string}}
   */
  getDPObjectSchemaRef(type) {
    if (!this.isDPObjectDefined(type)) {
      throw new InvalidDPObjectTypeError(type, this);
    }

    return { $ref: `${this.getJsonSchemaId()}#/dpObjectsDefinition/${type}` };
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
   * @return {{$schema: string, name: string,
   *           version: number, dpObjectsDefinition: Object<string, Object>,
   *           [definitions]: Object<string, Object>}}
   */
  toJSON() {
    const json = {
      $schema: this.getJsonMetaSchema(),
      name: this.getName(),
      version: this.getVersion(),
      dpObjectsDefinition: this.getDPObjectsDefinition(),
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
