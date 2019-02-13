const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const hash = require('../util/hash');
const { encode } = require('../util/serializer');

class DPObject {
  /**
   * @param {Object} rawDPObject
   */
  constructor(rawDPObject) {
    const data = Object.assign({}, rawDPObject);

    this.id = null;

    if (Object.prototype.hasOwnProperty.call(rawDPObject, '$type')) {
      this.type = rawDPObject.$type;
      delete data.$type;
    }

    if (Object.prototype.hasOwnProperty.call(rawDPObject, '$scopeId')) {
      this.scopeId = rawDPObject.$scopeId;
      delete data.$scopeId;
    }

    if (Object.prototype.hasOwnProperty.call(rawDPObject, '$scope')) {
      this.scope = rawDPObject.$scope;
      delete data.$scope;
    }

    if (Object.prototype.hasOwnProperty.call(rawDPObject, '$action')) {
      this.action = rawDPObject.$action;
      delete data.$action;
    }

    if (Object.prototype.hasOwnProperty.call(rawDPObject, '$rev')) {
      this.revision = rawDPObject.$rev;
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
   * @return {DPObject}
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
   * @return DPObject
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
   * @return {DPObject}
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
   * @return {DPObject}
   */
  set(path, value) {
    lodashSet(this.data, path, value);

    return this;
  }

  /**
   * Return DPObject as plain object
   *
   * @return {{$type: string,
   *           $scope: string,
   *           $scopeId: string,
   *           $rev: number,
   *           $action: number}}
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
   * Return serialized DPObject
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

DPObject.ACTIONS = {
  CREATE: 0,
  UPDATE: 1,
  DELETE: 2,
};

DPObject.DEFAULTS = {
  REVISION: 0,
  ACTION: DPObject.ACTIONS.CREATE,
};

DPObject.SYSTEM_PREFIX = '$';

module.exports = DPObject;
