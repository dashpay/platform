const bs58 = require('bs58');

const DPObject = require('@dashevo/dpp/lib/object/DPObject');

const SVObject = require('../../stateView/object/SVObject');
const Reference = require('../revisions/Reference');

const createRevisions = require('../revisions/createRevisions');

const InvalidWhereError = require('./errors/InvalidWhereError');
const InvalidOrderByError = require('./errors/InvalidOrderByError');
const InvalidLimitError = require('./errors/InvalidLimitError');
const InvalidStartAtError = require('./errors/InvalidStartAtError');
const InvalidStartAfterError = require('./errors/InvalidStartAfterError');
const AmbiguousStartError = require('./errors/AmbiguousStartError');

/**
 * @param {string} id
 * @return {string}
 */
function createDocumentIdFromObjectId(id) {
  const svObjectIdBuffer = Buffer.from(id, 'hex');

  return bs58.encode(svObjectIdBuffer);
}

/**
 * @param {SVObject} svObject
 * @return {string}
 */
function createDocumentIdFromSVObject(svObject) {
  const id = svObject.getDPObject().getId();

  return createDocumentIdFromObjectId(id);
}

class SVObjectMongoDbRepository {
  /**
   * @param {Db} mongoClient
   * @param {sanitizer} sanitizer
   * @param {string} objectType
   */
  constructor(mongoClient, sanitizer, objectType) {
    this.mongoClient = mongoClient.collection(`objects_${objectType}`);
    this.sanitizer = sanitizer;
  }

  /**
   * Find SV Object by id
   *
   * @param {string} id
   * @returns {Promise<SVObject>}
   */
  async find(id) {
    const documentId = createDocumentIdFromObjectId(id);

    const result = await this.mongoClient.findOne({ _id: documentId });

    if (!result) {
      return null;
    }

    return this.createSVObject(result);
  }

  /**
   * Find all objects by `reference.stHash`
   *
   * @param {string} stHash
   * @returns {Promise<SVObject[]>}
   */
  async findAllBySTHash(stHash) {
    const result = await this.mongoClient
      .find({ 'reference.stHash': stHash })
      .toArray();

    return result.map(rawObject => this.createSVObject(rawObject));
  }

  /**
   * Fetch SV Objects
   *
   * @param options
   * @param options.where
   * @param options.limit
   * @param options.startAt
   * @param options.startAfter
   * @param options.orderBy
   *
   * @returns {Promise<SVObject[]>}
   */
  async fetch(options = {}) {
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
      typeof options.startAt !== 'undefined'
      && typeof options.startAfter !== 'undefined'
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

    query = Object.assign({ isDeleted: false }, query);

    const results = await this.mongoClient.find(query, opts).toArray();

    return results.map(document => this.createSVObject(document));
  }

  /**
   * Store SVObject entity
   *
   * @param {SVObject} svObject
   * @returns {Promise}
   */
  store(svObject) {
    return this.mongoClient.updateOne(
      { _id: createDocumentIdFromSVObject(svObject) },
      { $set: this.sanitizer.sanitize(svObject.toJSON()) },
      { upsert: true },
    );
  }

  /**
   * Delete SVObject entity
   *
   * @param {SVObject} svObject
   * @returns {Promise}
   */
  async delete(svObject) {
    return this.mongoClient.deleteOne({
      _id: createDocumentIdFromSVObject(svObject),
    });
  }

  /**
   * @private
   * @return {SVObject}
   */
  createSVObject({
    userId,
    isDeleted,
    dpObject: sanitizedDPObject,
    reference,
    previousRevisions,
  }) {
    const rawDPObject = this.sanitizer.unsanitize(sanitizedDPObject);

    return new SVObject(
      userId,
      new DPObject(rawDPObject),
      new Reference(reference),
      isDeleted,
      createRevisions(previousRevisions),
    );
  }

  /**
   * @private
   * @param {object} obj
   * @returns {boolean}
   */
  isObject(obj) {
    return obj === Object(obj);
  }

  /**
   * @private
   * @param {number} num
   * @returns {boolean}
   */
  isNumber(num) {
    return typeof num === 'number';
  }
}

module.exports = SVObjectMongoDbRepository;
