const hash = require('../../dash-schema/lib/hash');

const InvalidDapObjectTypeError = require('./errors/InvalidDapObjectTypeError');

class DapContract {
  /**
   * @param {string} name
   * @param {Object<string, Object>} dapObjectsDefinition
   */
  constructor(name, dapObjectsDefinition) {
    this.setName(name);
    this.setVersion(DapContract.DEFAULTS.VERSION);
    this.setSchema(DapContract.DEFAULTS.SCHEMA);
    this.setDapObjectsDefinition(dapObjectsDefinition);
  }

  /**
   *
   * @param {string} name
   * @return {DapContract}
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
   * @return {DapContract}
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
  setSchema(schema) {
    this.$schema = schema;
  }

  /**
   *
   * @return {string}
   */
  getSchema() {
    return this.$schema;
  }

  /**
   *
   * @param {Object<string, Object>} dapObjectsDefinition
   * @return {DapContract}
   */
  setDapObjectsDefinition(dapObjectsDefinition) {
    this.dapObjectsDefinition = dapObjectsDefinition;

    return this;
  }

  /**
   *
   * @return {Object<string, Object>}
   */
  getDapObjectsDefinition() {
    return this.dapObjectsDefinition;
  }

  /**
   * @template TDefinitions {Object}
   * @param {TDefinitions} definitions
   */
  setDefinitions(definitions) {
    this.definitions = definitions;
  }

  /**
   * @return {Object}
   */
  getDefinitions() {
    return this.definitions;
  }

  /**
   * Returns true if object type is defined in this dap contract
   *
   * @param {string} type
   * @return {boolean}
   */
  isDapObjectDefined(type) {
    return Object.prototype.hasOwnProperty.call(this.dapObjectsDefinition, type);
  }

  /**
   *
   * @param {string} type
   * @param {object} schema
   * @return {DapContract}
   */
  setDapObjectSchema(type, schema) {
    this.dapObjectsDefinition[type] = schema;

    return this;
  }

  /**
   *
   * @param {string} type
   * @return {Object}
   */
  getDapObjectSchema(type) {
    if (!this.isDapObjectDefined(type)) {
      throw new InvalidDapObjectTypeError(this, type);
    }

    return this.dapObjectsDefinition[type];
  }

  /**
   * @param {string} type
   * @return {{$ref: string}}
   */
  getDapObjectSchemaRef(type) {
    if (!this.isDapObjectDefined(type)) {
      throw new InvalidDapObjectTypeError(this, type);
    }

    return { $ref: `dap-contract#/dapObjectsDefinition/${type}` };
  }

  getId() {
    const serializedDapContract = DapContract.serializer.encode(this.toJSON());
    return hash(serializedDapContract);
  }

  toJSON() {
    const json = {};

    for (const name of Object.getOwnPropertyNames(this)) {
      json[name] = this[name];
    }

    return json;
  }

  /**
   * Serialize Dap Contract
   *
   * @return {Buffer}
   */
  serialize() {
    return DapContract.serializer.encode(this.toJSON());
  }

  static fromObject(object) {
    const errors = DapContract.validateStructure(object);

    if (errors.length) {
      throw new Error(errors);
    }

    const dapContract = new DapContract(object.name, object.dapObjectsDefinition);

    dapContract.setSchema(object.$schema);
    dapContract.setVersion(object.version);

    if (object.definitions) {
      dapContract.setDefinitions(object.definitions);
    }

    return dapContract;
  }

  /**
   *
   * @param {Buffer|string} payload
   * @return {DapContract}
   */
  static fromSerialized(payload) {
    const object = DapContract.serializer.encode(payload);
    return DapContract.fromObject(object);
  }

  /**
   * Set serializer
   *
   * @param {serializer} serializer
   */
  static setSerializer(serializer) {
    DapContract.serializer = serializer;
  }

  /**
   * Set structure validator
   *
   * @param {Function} validator
   */
  static setStructureValidator(validator) {
    DapContract.validateStructure = validator;
  }
}

DapContract.DEFAULTS = {
  VERSION: 1,
  SCHEMA: 'https://schema.dash.org/platform-4-0-0/system/meta/dap-contract',
};

module.exports = DapContract;
