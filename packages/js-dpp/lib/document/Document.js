const bs58 = require('bs58');
const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const DataIsNotAllowedWithActionDeleteError = require('./errors/DataIsNotAllowedWithActionDeleteError');

const hash = require('../util/hash');
const { encode } = require('../util/serializer');

class Document {
  /**
   * @param {RawDocument} rawDocument
   */
  constructor(rawDocument) {
    const data = Object.assign({}, rawDocument);

    this.id = null;

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$type')) {
      this.type = rawDocument.$type;
      delete data.$type;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$scopeId')) {
      this.scopeId = rawDocument.$scopeId;
      delete data.$scopeId;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$scope')) {
      this.scope = rawDocument.$scope;
      delete data.$scope;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$action')) {
      this.action = rawDocument.$action;
      delete data.$action;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$rev')) {
      this.revision = rawDocument.$rev;
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
      this.id = bs58.encode(
        hash(this.scope + this.scopeId),
      );
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
   * @return {Document}
   */
  setAction(action) {
    if (action === Document.ACTIONS.DELETE && Object.keys(this.data).length !== 0) {
      throw new DataIsNotAllowedWithActionDeleteError(this);
    }

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
   * @return Document
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
   * @return {Document}
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
   * @return {Document}
   */
  set(path, value) {
    if (this.action === Document.ACTIONS.DELETE) {
      throw new DataIsNotAllowedWithActionDeleteError(this);
    }

    lodashSet(this.data, path, value);

    return this;
  }

  /**
   * Return Document as plain object
   *
   * @return {RawDocument}
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
   * Return serialized Document
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
    return hash(this.serialize()).toString('hex');
  }
}

Document.ACTIONS = {
  CREATE: 0,
  UPDATE: 1,
  DELETE: 2,
};

Document.DEFAULTS = {
  REVISION: 0,
  ACTION: Document.ACTIONS.CREATE,
};

Document.SYSTEM_PREFIX = '$';

module.exports = Document;
