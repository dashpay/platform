const hash = require('../util/hash');
const serializer = require('../util/serializer');
const entropy = require('../util/entropy');

class DapObject {
  /**
   * @param {DapContract} dapContract
   * @param {string} blockchainUserId
   * @param {string} type
   * @param {object} [data]
   */
  constructor(dapContract, blockchainUserId, type, data = {}) {
    // TODO Strange to pass system fields as data, but then prevent to use setData for them

    const userData = Object.assign({}, data);

    if (Object.prototype.hasOwnProperty.call(userData, '$type')) {
      this.type = userData.$type;
      delete userData.$type;
    } else {
      this.type = type;
    }

    if (Object.prototype.hasOwnProperty.call(userData, '$scopeId')) {
      this.scopeId = userData.$scopeId;
      delete userData.$scopeId;
    } else {
      this.scopeId = entropy.generate();
    }

    if (Object.prototype.hasOwnProperty.call(userData, '$scope')) {
      this.scope = userData.$scope;
      delete userData.$scope;
    } else {
      this.scope = hash(dapContract.getId() + blockchainUserId);
    }

    if (Object.prototype.hasOwnProperty.call(userData, '$action')) {
      this.action = userData.$action;
      delete userData.$action;
    } else {
      this.action = DapObject.ACTIONS.CREATE;
    }

    if (Object.prototype.hasOwnProperty.call(userData, '$rev')) {
      this.revision = userData.$rev;
      delete userData.$rev;
    } else {
      this.revision = DapObject.DEFAULTS.REVISION;
    }

    this.setData(userData);
  }

  /**
   * Get ID
   *
   * @return {string}
   */
  getId() {
    if (!this.id) {
      this.id = hash(this.scope + this.scopeId);
    }

    return this.id;
  }

  /**
   * Get type
   *
   * @return {string}
   */
  getType() {
    return this.type;
  }

  /**
   * Set action
   *
   * @param {number} action
   * @return {DapObject}
   */
  setAction(action) {
    this.action = action;

    return this;
  }

  /**
   * Get action
   *
   * @return {number}
   */
  getAction() {
    return this.action;
  }

  /**
   * Set revision
   *
   * @param {number} revision
   * @return DapObject
   */
  setRevision(revision) {
    this.revision = revision;

    return this;
  }

  /**
   * Get revision
   *
   * @return {number}
   */
  getRevision() {
    return this.revision;
  }

  /**
   * Set data
   *
   * @param {Object} data
   * @return {DapObject}
   */
  setData(data) {
    this.data = {};

    Object.entries(data).forEach(([name, value]) => this.set(name, value));

    return this;
  }

  /**
   * Get data
   *
   * @return {Object}
   */
  getData() {
    return this.data;
  }

  /**
   * Retrieves the field specified by {path}
   *
   * @param {string} path
   * @return {*}
   */
  get(path) {
    // TODO implement path
    return this.data[path];
  }

  /**
   * Set the field specified by {path}
   *
   * @param path
   * @param value
   * @return {DapObject}
   */
  set(path, value) {
    if (path[0] === '$') {
      throw new Error();
    }

    // TODO implement path
    this.data[path] = value;

    return this;
  }

  /**
   * Return Dap Object as plain object
   *
   * @return {Object}
   */
  toJSON() {
    return {
      $scope: this.scope,
      $scopeId: this.scopeId,
      $type: this.getType(),
      $rev: this.getRevision(),
      $action: this.getAction(),
      ...this.getData(),
    };
  }

  /**
   * Return serialized Dap Object
   *
   * @return {Buffer}
   */
  serialize() {
    return serializer.encode(this.toJSON());
  }

  /**
   * Returns hex string with object hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize());
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
