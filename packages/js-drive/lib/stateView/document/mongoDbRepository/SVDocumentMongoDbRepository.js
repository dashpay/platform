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
    this.databaseName = mongoDatabase.databaseName;
    this.collectionName = this.getCollectionName();
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
   * Drops mongoDatabase collection
   * @returns {Promise<boolean>}
   */
  async removeCollection() {
    return this.mongoCollection.drop();
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
   * @param {MongoDBTransaction} [stateViewTransaction]
   * @returns {Promise<SVDocument>}
   */
  async find(id, stateViewTransaction = undefined) {
    const findQuery = { _id: id };

    let result;
    if (stateViewTransaction) {
      const transactionFunction = async (mongoClient, session) => (
        mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .findOne(findQuery, { session })
      );

      result = await stateViewTransaction.runWithTransaction(transactionFunction);
    } else {
      result = await this.mongoCollection.findOne(findQuery);
    }

    if (!result) {
      return null;
    }

    return this.createSVDocument(result);
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
   * @param {MongoDBTransaction} [stateViewTransaction]
   *
   * @returns {Promise<SVDocument[]>}
   * @throws {InvalidQueryError}
   */
  async fetch(query = {}, stateViewTransaction = undefined) {
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

    let results;

    if (stateViewTransaction) {
      const transactionFunction = async (mongoClient, session) => {
        findOptions = Object.assign({}, findOptions, { session });

        return mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .find(findQuery, findOptions).toArray();
      };

      results = await stateViewTransaction.runWithTransaction(transactionFunction);
    } else {
      results = await this.mongoCollection.find(findQuery, findOptions).toArray();
    }

    return results.map(document => this.createSVDocument(document));
  }

  /**
   * Store SVDocument entity
   *
   * @param {SVDocument} svDocument
   * @param {MongoDBTransaction} [stateViewTransaction]
   * @returns {Promise}
   */
  store(svDocument, stateViewTransaction = undefined) {
    const filter = { _id: svDocument.getDocument().getId() };
    const update = { $set: svDocument.toJSON() };
    let updateOptions = { upsert: true };

    if (stateViewTransaction) {
      const transactionFunction = async (mongoClient, session) => {
        updateOptions = Object.assign({}, updateOptions, { session });

        return mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .updateOne(
            filter,
            update,
            updateOptions,
          );
      };

      return stateViewTransaction.runWithTransaction(transactionFunction);
    }

    return this.mongoCollection.updateOne(
      filter,
      update,
      updateOptions,
    );
  }

  /**
   * Delete SVDocument entity
   *
   * @param {SVDocument} svDocument
   * @param {MongoDBTransaction} [stateViewTransaction]
   * @returns {Promise}
   */
  async delete(svDocument, stateViewTransaction = undefined) {
    const filter = { _id: svDocument.getDocument().getId() };

    if (stateViewTransaction) {
      const transactionFunction = async (mongoClient, session) => (
        mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .deleteOne(filter, { session })
      );

      return stateViewTransaction.runWithTransaction(transactionFunction);
    }

    return this.mongoCollection.deleteOne(filter);
  }

  /**
   * @private
   * @return {SVDocument}
   */
  createSVDocument({
    entropy,
    contractId,
    userId,
    data: storedData,
    isDeleted,
    reference,
    action,
    currentRevision,
    previousRevisions,
  }) {
    const rawDocument = Object.assign({}, storedData);

    rawDocument.$type = this.documentType;
    rawDocument.$entropy = entropy;
    rawDocument.$contractId = contractId;
    rawDocument.$userId = userId;
    rawDocument.$rev = currentRevision.revision;

    const document = new Document(rawDocument);
    document.setAction(action);

    return new SVDocument(
      document,
      new Reference(reference),
      isDeleted,
      createRevisions(previousRevisions),
    );
  }
}

module.exports = SVDocumentMongoDbRepository;
