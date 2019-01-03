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
   * @param {string} objectType
   */
  constructor(mongoClient, objectType) {
    this.mongoClient = mongoClient.collection(`dapObjects_${objectType}`);
  }

  /**
   * Find DapObject by id
   *
   * @param {string} id
   * @returns {Promise<DapObject>}
   */
  async find(id) {
    const result = await this.mongoClient.findOne({ _id: id });

    if (!result) {
      return null;
    }

    return this.toDapObject(result);
  }

  /**
   * Find all dap objects by `reference.stHeaderHash`
   *
   * @param {string} stHeaderHash
   * @returns {Promise<DapObject[]>}
   */
  async findAllBySTHeaderHash(stHeaderHash) {
    const result = await this.mongoClient
      .find({ 'reference.stHeaderHash': stHeaderHash })
      .toArray();

    return result.map(document => this.toDapObject(document));
  }

  /**
   * Fetch DapObjects
   *
   * @param options
   * @param options.where
   * @param options.limit
   * @param options.startAt
   * @param options.startAfter
   * @param options.orderBy
   * @returns {Promise<DapObject[]>}
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
    return results.map(document => this.toDapObject(document));
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
  toDapObject(objectFromDb) {
    const data = {
      ...objectFromDb.data,
      objtype: objectFromDb.type,
      pver: objectFromDb.protocolVersion,
      idx: objectFromDb.idx,
      rev: objectFromDb.revision,
      act: objectFromDb.action,
    };

    const {
      blockchainUserId,
      isDeleted,
      reference: referenceData,
      previousRevisions: previousRevisionsData,
    } = objectFromDb;

    const reference = new Reference(
      referenceData.blockHash,
      referenceData.blockHeight,
      referenceData.stHeaderHash,
      referenceData.stPacketHash,
      referenceData.objectHash,
    );

    return new DapObject(
      blockchainUserId,
      data,
      reference,
      isDeleted,
      this.toPreviousRevisions(
        previousRevisionsData,
      ),
    );
  }

  /**
   * @private
   * @param {array} previousRevisionsData
   * @returns {{revision: number, reference: Reference}[]}
   */
  toPreviousRevisions(previousRevisionsData = []) {
    return previousRevisionsData.map((revisionItem) => {
      const previousRevision = revisionItem.revision;
      const previousReferenceData = revisionItem.reference;
      return {
        revision: previousRevision,
        reference: new Reference(
          previousReferenceData.blockHash,
          previousReferenceData.blockHeight,
          previousReferenceData.stHeaderHash,
          previousReferenceData.stPacketHash,
          previousReferenceData.objectHash,
        ),
      };
    });
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

module.exports = DapObjectMongoDbRepository;
