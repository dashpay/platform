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

    this.id = undefined;
    this.action = undefined;

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$type')) {
      this.type = rawDocument.$type;
      delete data.$type;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$entropy')) {
      this.entropy = rawDocument.$entropy;
      delete data.$entropy;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$contractId')) {
      this.contractId = rawDocument.$contractId;
      delete data.$contractId;
    }

    if (Object.prototype.hasOwnProperty.call(rawDocument, '$userId')) {
      this.userId = rawDocument.$userId;
      delete data.$userId;
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
        hash(this.contractId + this.userId + this.type + this.entropy),
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
   * Get Data Contract ID
   *
   * @return {string}
   */
  getDataContractId() {
    return this.contractId;
  }

  /**
   * Get User ID
   *
   * @return {string}
   */
  getUserId() {
    return this.userId;
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
      $contractId: this.getDataContractId(),
      $userId: this.getUserId(),
      $entropy: this.entropy,
      $rev: this.getRevision(),
      ...this.getData(),
    };
  }

  /**
   * Return serialized Document
   *
   * @return {Buffer}
   */
  serialize() {
    const json = this.toJSON();

    return encode(json);
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
  CREATE: 1,
  REPLACE: 2,
  DELETE: 4,
};

Document.DEFAULTS = {
  REVISION: 1,
  ACTION: Document.ACTIONS.CREATE,
};

Document.SYSTEM_PREFIX = '$';

module.exports = Document;
