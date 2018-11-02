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

  setType(type) {
    this.$$type = type;

    return this;
  }

  getType() {
    return this.$$type;
  }

  getAction() {
    return this.$$action;
  }

  setAction(action) {
    this.$$action = action;

    return this;
  }

  toJSON() {
    const json = {};

    for (const name of Object.getOwnPropertyNames(this)) {
      json[name] = this[name];
    }

    return json;
  }

  static fromObject(object) {
    const errors = DapObject.validateStructure(object);

    if (errors.length) {
      throw new Error(errors);
    }

    return new DapObject(object.$$type, object);
  }

  /**
   *
   * @param {Buffer|string} payload
   * @return {DapObject}
   */
  static fromSerialized(payload) {
    const object = DapObject.serializer.decode(payload);
    return DapObject.fromObject(object);
  }

  static setSerializer(serializer) {
    DapObject.serializer = serializer;
  }

  /**
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
