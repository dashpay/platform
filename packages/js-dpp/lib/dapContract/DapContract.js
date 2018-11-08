const hash = require('../hash');

const InvalidDapObjectTypeError = require('./errors/InvalidDapObjectTypeError');
const InvalidDapContractStructureError = require('./errors/InvalidDapContractStructureError');

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
    this.setDefinitions({});
  }

  /**
   * Calculate Dap Contract ID
   *
   * @return {string}
   */
  getId() {
    const serializedDapContract = DapContract.serializer.encode(this.toJSON());
    return hash(serializedDapContract);
  }

  /**
   * Get Schema ID
   *
   * @return {string}
   */
  getSchemaId() {
    return 'dap-contract';
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
   * @param {Object<string, Object>} definitions
   */
  setDefinitions(definitions) {
    this.definitions = definitions;
  }

  /**
   * @return {Object<string, Object>}
   */
  getDefinitions() {
    return this.definitions;
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
   * @return {Object}
   */
  getDapObjectSchema(type) {
    if (!this.isDapObjectDefined(type)) {
      throw new InvalidDapObjectTypeError(type, this);
    }

    return this.dapObjectsDefinition[type];
  }

  /**
   * @param {string} type
   * @return {{$ref: string}}
   */
  getDapObjectSchemaRef(type) {
    if (!this.isDapObjectDefined(type)) {
      throw new InvalidDapObjectTypeError(type, this);
    }

    return { $ref: `${this.getSchemaId()}#/dapObjectsDefinition/${type}` };
  }

  /**
   * Return Dap Contract as plain object
   *
   * @return {{$schema: string, name: string,
   *           version: number, dapObjectsDefinition: Object<string, Object>,
   *           [definitions]: Object<string, Object>}}
   */
  toJSON() {
    const json = {
      $schema: this.getSchema(),
      name: this.getName(),
      version: this.getVersion(),
      dapObjectsDefinition: this.getDapObjectsDefinition(),
    };

    const definitions = this.getDefinitions();

    if (Object.getOwnPropertyNames(definitions).length) {
      json.definitions = definitions;
    }

    return json;
  }

  /**
   * Return serialized Dap Contract
   *
   * @return {Buffer}
   */
  serialize() {
    return DapContract.serializer.encode(this.toJSON());
  }

  /**
   * Returns hex string with contract hash
   *
   * @return {string}
   */
  hash() {
    return DapContract.hashingFunction(this.serialize());
  }

  /**
   * Create Dap Contract from plain object
   *
   * @param object
   * @return {DapContract}
   */
  static fromObject(object) {
    const errors = DapContract.validateStructure(object);

    if (errors.length) {
      throw new InvalidDapContractStructureError(errors, object);
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
   * Create Dap Contract from string/buffer
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

  /**
   * Set hashing function
   *
   * @param {function(Buffer):string}  hashingFunction
   */
  static setHashingFunction(hashingFunction) {
    DapContract.hashingFunction = hashingFunction;
  }
}

DapContract.DEFAULTS = {
  VERSION: 1,
  SCHEMA: 'https://schema.dash.org/platform-4-0-0/system/meta/dap-contract',
};

DapContract.SCHEMA_ID = 'dap-contract';

module.exports = DapContract;
