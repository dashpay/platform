const InvalidDapObjectStructureError = require('./errors/InvalidDapObjectStructureError');

class DapObject {
  /**
   * @param {string} type
   * @param {object} [data]
   */
  constructor(type, data = {}) {
    this.setType(type);
    this.setAction(DapObject.ACTIONS.CREATE);
    this.setRevision(DapObject.DEFAULTS.REVISION);

    Object.assign(this, data);
  }

  /**
   * Set type
   *
   * @param {string }type
   * @return {DapObject}
   */
  setType(type) {
    this.$$type = type;

    return this;
  }

  /**
   * Get type
   *
   * @return {string}
   */
  getType() {
    return this.$$type;
  }

  /**
   * Set action
   *
   * @param {number} action
   * @return {DapObject}
   */
  setAction(action) {
    this.$$action = action;

    return this;
  }

  /**
   * Get action
   *
   * @return {number}
   */
  getAction() {
    return this.$$action;
  }

  /**
   * Set revision
   *
   * @param {number} revision
   * @return DapObject
   */
  setRevision(revision) {
    this.$$revision = revision;

    return this.$$revision;
  }

  /**
   * Get revision
   *
   * @return {number}
   */
  getRevision() {
    return this.$$revision;
  }

  /**
   * Return Dap Object as plain object
   *
   * @return {Object}
   */
  toJSON() {
    const json = {};

    Object.getOwnPropertyNames(this).forEach((name) => {
      json[name] = this[name];
    });

    return json;
  }

  /**
   * Return serialized Dap Object
   *
   * @return {Buffer}
   */
  serialize() {
    return DapObject.serializer.encode(this.toJSON());
  }

  /**
   * Returns hex string with object hash
   *
   * @return {string}
   */
  hash() {
    return DapObject.hashingFunction(this.serialize());
  }

  /**
   * Create Dap Object from plain object
   *
   * @param object
   * @return {DapObject}
   */
  static fromObject(object) {
    const errors = DapObject.validateStructure(object);

    if (errors.length) {
      throw new InvalidDapObjectStructureError(errors, object);
    }

    return new DapObject(object.$$type, object);
  }

  /**
   * Create Dap Object from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DapObject}
   */
  static fromSerialized(payload) {
    const object = DapObject.serializer.decode(payload);
    return DapObject.fromObject(object);
  }

  /**
   * Set serializer
   *
   * @param {serializer} serializer
   */
  static setSerializer(serializer) {
    DapObject.serializer = serializer;
  }

  /**
   * Set structure validator
   *
   * @param {Function} validator
   */
  static setStructureValidator(validator) {
    DapObject.validateStructure = validator;
  }

  /**
   * Set hashing function
   *
   * @param {function(Buffer):string}  hashingFunction
   */
  static setHashingFunction(hashingFunction) {
    DapObject.hashingFunction = hashingFunction;
  }
}

DapObject.ACTIONS = {
  CREATE: 0,
  UPDATE: 1,
  DELETE: 2,
};

DapObject.DEFAULTS = {
  REVISION: 0,
};

module.exports = DapObject;
