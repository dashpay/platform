const Document = require('@dashevo/dpp/lib/document/Document');

const SVDocument = require('./SVDocument');
const Reference = require('../revisions/Reference');

const createRevisions = require('../revisions/createRevisions');

const InvalidWhereError = require('./errors/InvalidWhereError');
const InvalidOrderByError = require('./errors/InvalidOrderByError');
const InvalidLimitError = require('./errors/InvalidLimitError');
const InvalidStartAtError = require('./errors/InvalidStartAtError');
const InvalidStartAfterError = require('./errors/InvalidStartAfterError');
const AmbiguousStartError = require('./errors/AmbiguousStartError');

class SVDocumentMongoDbRepository {
  /**
   * @param {Db} mongoClient
   * @param {sanitizer} sanitizer
   * @param {string} documentType
   */
  constructor(mongoClient, sanitizer, documentType) {
    this.mongoClient = mongoClient.collection(`documents_${documentType}`);
    this.sanitizer = sanitizer;
  }

  /**
   * Find SVDocument by id
   *
   * @param {string} id
   * @returns {Promise<SVDocument>}
   */
  async find(id) {
    const result = await this.mongoClient.findOne({ _id: id });

    if (!result) {
      return null;
    }

    return this.createSVDocument(result);
  }

  /**
   * Find all documents by `reference.stHash`
   *
   * @param {string} stHash
   * @returns {Promise<SVDocument[]>}
   */
  async findAllBySTHash(stHash) {
    const result = await this.mongoClient
      .find({ 'reference.stHash': stHash })
      .toArray();

    return result.map(rawDocument => this.createSVDocument(rawDocument));
  }

  /**
   * Fetch SVDocuments
   *
   * @param options
   * @param options.where
   * @param options.limit
   * @param options.startAt
   * @param options.startAfter
   * @param options.orderBy
   *
   * @returns {Promise<SVDocument[]>}
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

    return results.map(document => this.createSVDocument(document));
  }

  /**
   * Store SVDocument entity
   *
   * @param {SVDocument} svDocument
   * @returns {Promise}
   */
  store(svDocument) {
    return this.mongoClient.updateOne(
      { _id: svDocument.getDocument().getId() },
      { $set: this.sanitizer.sanitize(svDocument.toJSON()) },
      { upsert: true },
    );
  }

  /**
   * Delete SVDocument entity
   *
   * @param {SVDocument} svDocument
   * @returns {Promise}
   */
  async delete(svDocument) {
    return this.mongoClient.deleteOne({
      _id: svDocument.getDocument().getId(),
    });
  }

  /**
   * @private
   * @return {SVDocument}
   */
  createSVDocument({
    userId,
    isDeleted,
    document: sanitizedDocument,
    reference,
    previousRevisions,
  }) {
    const rawDocument = this.sanitizer.unsanitize(sanitizedDocument);

    return new SVDocument(
      userId,
      new Document(rawDocument),
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

module.exports = SVDocumentMongoDbRepository;
