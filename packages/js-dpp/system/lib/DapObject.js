/**
 * @class DapObject
 * @property $$type
 */
class DapObject {
  /**
   * @param {string} type
   * @param {object} [data]
   */
  constructor(type, data = {}) {
    this.setAction(DapObject.ACTIONS.CREATE);
    this.setType(type);

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
   * Return Dap Object as plain object
   *
   * @return {Object}
   */
  toJSON() {
    const json = {};

    for (const name of Object.getOwnPropertyNames(this)) {
      json[name] = this[name];
    }

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
   * Create Dap Object from plain object
   *
   * @param object
   * @return {DapObject}
   */
  static fromObject(object) {
    const errors = DapObject.validateStructure(object);

    if (errors.length) {
      throw new Error(errors);
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
}

DapObject.ACTIONS = {
  CREATE: 0,
  UPDATE: 1,
  DELETE: 2,
};

module.exports = DapObject;
