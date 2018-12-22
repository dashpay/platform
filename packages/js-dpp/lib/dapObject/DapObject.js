const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const hash = require('../util/hash');
const { encode } = require('../util/serializer');

class DapObject {
  /**
   * @param {Object} rawDapObject
   */
  constructor(rawDapObject) {
    const data = Object.assign({}, rawDapObject);

    this.id = null;

    if (Object.prototype.hasOwnProperty.call(rawDapObject, '$type')) {
      this.type = rawDapObject.$type;
      delete data.$type;
    }

    if (Object.prototype.hasOwnProperty.call(rawDapObject, '$scopeId')) {
      this.scopeId = rawDapObject.$scopeId;
      delete data.$scopeId;
    }

    if (Object.prototype.hasOwnProperty.call(rawDapObject, '$scope')) {
      this.scope = rawDapObject.$scope;
      delete data.$scope;
    }

    if (Object.prototype.hasOwnProperty.call(rawDapObject, '$action')) {
      this.action = rawDapObject.$action;
      delete data.$action;
    }

    if (Object.prototype.hasOwnProperty.call(rawDapObject, '$rev')) {
      this.revision = rawDapObject.$rev;
      delete data.$rev;
    }

    this.setData(data);
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

    Object.entries(data)
      .forEach(([name, value]) => this.set(name, value));

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
    return lodashGet(this.data, path);
  }

  /**
   * Set the field specified by {path}
   *
   * @param {string} path
   * @param {*} value
   * @return {DapObject}
   */
  set(path, value) {
    lodashSet(this.data, path, value);

    return this;
  }

  /**
   * Return Dap Object as plain object
   *
   * @return {Object}
   */
  toJSON() {
    return {
      $type: this.getType(),
      $scope: this.scope,
      $scopeId: this.scopeId,
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
    return encode(this.toJSON());
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
  ACTION: DapObject.ACTIONS.CREATE,
};

DapObject.SYSTEM_PREFIX = '$';

module.exports = DapObject;
