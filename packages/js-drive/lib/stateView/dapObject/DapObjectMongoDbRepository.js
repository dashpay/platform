const DapObject = require('./DapObject');
const Reference = require('../Reference');
const InvalidWhereError = require('./errors/InvalidWhereError');
const InvalidOrderByError = require('./errors/InvalidOrderByError');
const InvalidLimitError = require('./errors/InvalidLimitError');
const InvalidStartAtError = require('./errors/InvalidStartAtError');
const InvalidStartAfterError = require('./errors/InvalidStartAfterError');
const AmbiguousStartError = require('./errors/AmbiguousStartError');

class DapObjectMongoDbRepository {
  /**
   * @param {Db} mongoClient
   */
  constructor(mongoClient) {
    this.mongoClient = mongoClient.collection('dapObjects');
  }

  /**
   * Find DapObject by id
   *
   * @param {string} id
   * @returns {Promise<DapObject>}
   */
  async find(id) {
    const result = await this.mongoClient.findOne({ _id: id });
    const { blockchainUserId, object: objectState, reference: referenceState } = result || {};
    return this.toDapObject(blockchainUserId, objectState, referenceState);
  }

  /**
   * Fetch DapObjects by type
   *
   * @param {string} type
   * @param options
   * @param options.where
   * @param options.limit
   * @param options.startAt
   * @param options.startAfter
   * @param options.orderBy
   * @returns {Promise<DapObject[]>}
   */
  async fetch(type, options = {}) {
    let query = {};
    let opts = {};

    if (this.isObject(options.where)) {
      query = Object.assign({}, query, options.where);
    } else if (typeof options.where !== 'undefined') {
      throw new InvalidWhereError();
    }

    if (this.isNumber(options.limit)) {
      opts = Object.assign({}, opts, { limit: options.limit });
    } else if (typeof options.limit !== 'undefined') {
      throw new InvalidLimitError();
    }

    if (
      typeof options.startAt !== 'undefined' &&
      typeof options.startAfter !== 'undefined'
    ) {
      throw new AmbiguousStartError();
    }

    if (this.isNumber(options.startAt)) {
      opts = Object.assign({}, opts, { skip: options.startAt - 1 });
    } else if (typeof options.startAt !== 'undefined') {
      throw new InvalidStartAtError();
    }

    if (this.isNumber(options.startAfter)) {
      opts = Object.assign({}, opts, { skip: options.startAfter });
    } else if (typeof options.startAfter !== 'undefined') {
      throw new InvalidStartAfterError();
    }

    if (this.isObject(options.orderBy)) {
      opts = Object.assign({}, opts, { sort: options.orderBy });
    } else if (typeof options.orderBy !== 'undefined') {
      throw new InvalidOrderByError();
    }

    query = Object.assign({}, query, { type });
    const results = await this.mongoClient.find(query, opts).toArray();
    return results.map((result) => {
      const { blockchainUserId, object: objectState, reference: referenceState } = result || {};
      return this.toDapObject(blockchainUserId, objectState, referenceState);
    });
  }

  /**
   * Store DapObject entity
   *
   * @param {DapObject} dapObject
   * @returns {Promise}
   */
  store(dapObject) {
    return this.mongoClient.updateOne(
      { _id: dapObject.getId() },
      { $set: dapObject.toJSON() },
      { upsert: true },
    );
  }

  /**
   * Delete DapObject entity
   *
   * @param dapObject
   * @returns {Promise}
   */
  async delete(dapObject) {
    return this.mongoClient.deleteOne({ _id: dapObject.getId() });
  }

  /**
   * @private
   * @return {DapObject}
   */
  // eslint-disable-next-line class-methods-use-this
  toDapObject(blockchainUserId, objectState = {}, referenceState = {}) {
    const reference = new Reference(
      referenceState.blockHash,
      referenceState.blockHeight,
      referenceState.stHeaderHash,
      referenceState.stPacketHash,
    );
    return new DapObject(blockchainUserId, objectState, reference);
  }

  /**
   * @private
   * @param {object} obj
   * @returns {boolean}
   */
  // eslint-disable-next-line class-methods-use-this
  isObject(obj) {
    return obj === Object(obj);
  }

  /**
   * @private
   * @param {number} num
   * @returns {boolean}
   */
  // eslint-disable-next-line class-methods-use-this
  isNumber(num) {
    return typeof num === 'number';
  }
}

module.exports = DapObjectMongoDbRepository;
