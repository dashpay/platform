const Document = require('@dashevo/dpp/lib/document/Document');

const SVDocument = require('../SVDocument');
const Reference = require('../../revisions/Reference');

const convertFieldName = require('./convertFieldName');

const createRevisions = require('../../revisions/createRevisions');

const InvalidQueryError = require('../errors/InvalidQueryError');

class SVDocumentMongoDbRepository {
  /**
   * @param {Db} mongoDatabase
   * @param {convertWhereToMongoDbQuery} convertWhereToMongoDbQuery
   * @param {validateQuery} validateQuery
   * @param {string} documentType
   */
  constructor(mongoDatabase, convertWhereToMongoDbQuery, validateQuery, documentType) {
    this.mongoDatabase = mongoDatabase;
    this.convertWhereToMongoDbQuery = convertWhereToMongoDbQuery;
    this.validateQuery = validateQuery;
    this.documentType = documentType;
    this.mongoCollection = mongoDatabase.collection(this.getCollectionName());
  }

  /**
   * Create new mongoDatabase collection
   *
   * @returns {Promise<*>}
   */
  async createCollection() {
    return this.mongoDatabase.createCollection(this.getCollectionName());
  }

  /**
   * Returns mongoDatabase collection name
   *
   * @private
   * @returns {string}
   */
  getCollectionName() {
    return `documents_${this.documentType}`;
  }

  /**
   * Find SVDocument by id
   *
   * @param {string} id
   * @returns {Promise<SVDocument>}
   */
  async find(id) {
    const result = await this.mongoCollection.findOne({ _id: id });

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
    const result = await this.mongoCollection
      .find({ 'reference.stHash': stHash })
      .toArray();

    return result.map(rawDocument => this.createSVDocument(rawDocument));
  }

  /**
   * Fetch SVDocuments
   *
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   *
   * @returns {Promise<SVDocument[]>}
   * @throws {InvalidQueryError}
   */
  async fetch(query = {}) {
    const result = this.validateQuery(query);

    if (!result.isValid()) {
      throw new InvalidQueryError(result.getErrors());
    }

    let findQuery = {};
    let findOptions = {};

    // Prepare find query
    if (query.where) {
      findQuery = this.convertWhereToMongoDbQuery(query.where);
    }

    findQuery = Object.assign({ isDeleted: false }, findQuery);

    // Prepare find options
    findOptions = Object.assign({}, findOptions, { limit: query.limit || 100 });

    if (query.startAt && query.startAt > 1) {
      findOptions = Object.assign({}, findOptions, { skip: query.startAt - 1 });
    }

    if (query.startAfter) {
      findOptions = Object.assign({}, findOptions, { skip: query.startAfter });
    }

    if (query.orderBy) {
      const sort = query.orderBy.map(([field, direction]) => {
        const mongoDbField = convertFieldName(field);

        return [mongoDbField, direction === 'asc' ? 1 : -1];
      });

      findOptions = Object.assign({}, findOptions, { sort });
    }

    const results = await this.mongoCollection.find(findQuery, findOptions).toArray();

    return results.map(document => this.createSVDocument(document));
  }

  /**
   * Store SVDocument entity
   *
   * @param {SVDocument} svDocument
   * @returns {Promise}
   */
  store(svDocument) {
    return this.mongoCollection.updateOne(
      { _id: svDocument.getDocument().getId() },
      { $set: svDocument.toJSON() },
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
    return this.mongoCollection.deleteOne({
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
    data: storedData,
    reference,
    scope,
    scopeId,
    action,
    currentRevision,
    previousRevisions,
  }) {
    const rawDocument = Object.assign({}, storedData);

    rawDocument.$scope = scope;
    rawDocument.$scopeId = scopeId;
    rawDocument.$action = action;
    rawDocument.$rev = currentRevision.revision;
    rawDocument.$type = this.documentType;
    rawDocument.$meta = {
      userId,
      stReference: {
        blockHash: reference.blockHash,
        blockHeight: reference.blockHeight,
        stHeaderHash: reference.stHash,
        stPacketHash: reference.stPacketHash,
      },
    };

    return new SVDocument(
      userId,
      new Document(rawDocument),
      new Reference(reference),
      isDeleted,
      createRevisions(previousRevisions),
    );
  }
}

module.exports = SVDocumentMongoDbRepository;
