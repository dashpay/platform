const Document = require('@dashevo/dpp/lib/document/Document');

const convertFieldName = require('./convertFieldName');

const InvalidQueryError = require('../errors/InvalidQueryError');

class DocumentMongoDbRepository {
  /**
   * @param {Db} mongoDatabase
   * @param {convertWhereToMongoDbQuery} convertWhereToMongoDbQuery
   * @param {validateQuery} validateQuery
   * @param {string} contractId
   * @param {string} documentType
   */
  constructor(mongoDatabase, convertWhereToMongoDbQuery, validateQuery, contractId, documentType) {
    this.mongoDatabase = mongoDatabase;
    this.convertWhereToMongoDbQuery = convertWhereToMongoDbQuery;
    this.validateQuery = validateQuery;
    this.documentType = documentType;
    this.contractId = contractId;
    this.databaseName = mongoDatabase.databaseName;
    this.collectionName = this.getCollectionName();
    this.mongoCollection = mongoDatabase.collection(this.getCollectionName());
  }

  /**
   * Create new mongoDatabase collection with indices
   * @param {{ key: object, unique: boolean, name: string }[]} [indices]
   * @returns {Promise<void>}
   */
  async createCollection(indices = undefined) {
    await this.mongoDatabase.createCollection(this.getCollectionName());

    if (indices) {
      await this.createIndices(indices);
    }
  }

  /**
   * Drops collection
   *
   * @returns {Promise<boolean>}
   */
  async removeCollection() {
    return this.mongoCollection.drop();
  }

  /**
   * Returns collection name
   *
   * @private
   * @returns {string}
   */
  getCollectionName() {
    return `documents_${this.documentType}`;
  }

  /**
   * Find Document by ID
   *
   * @param {string} id
   * @param {MongoDBTransaction} [transaction]
   * @returns {Promise<Document>}
   */
  async find(id, transaction = undefined) {
    const findQuery = { _id: id };

    let result;
    if (transaction) {
      const transactionFunction = async (mongoClient, session) => (
        mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .findOne(findQuery, { session })
      );

      result = await transaction.runWithTransaction(transactionFunction);
    } else {
      result = await this.mongoCollection.findOne(findQuery);
    }

    if (!result) {
      return null;
    }

    return this.createDppDocument(result);
  }

  /**
   * Fetch Documents
   *
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   * @param {Object} [documentSchema]
   * @param {MongoDBTransaction} [transaction]
   *
   * @returns {Promise<Document[]>}
   * @throws {InvalidQueryError}
   */

  async fetch(query = {}, documentSchema = {}, transaction = undefined) {
    const result = this.validateQuery(query, documentSchema);

    if (!result.isValid()) {
      throw new InvalidQueryError(result.getErrors());
    }

    let findQuery = {};
    let findOptions = {};

    // Prepare find query
    if (query.where) {
      findQuery = this.convertWhereToMongoDbQuery(query.where);
    }

    // Prepare find options
    findOptions = { ...findOptions, limit: query.limit || 100 };

    if (query.startAt && query.startAt > 1) {
      findOptions = { ...findOptions, skip: query.startAt - 1 };
    }

    if (query.startAfter) {
      findOptions = { ...findOptions, skip: query.startAfter };
    }

    if (query.orderBy) {
      const sort = query.orderBy.map(([field, direction]) => {
        const mongoDbField = convertFieldName(field);

        return [mongoDbField, direction === 'asc' ? 1 : -1];
      });

      findOptions = { ...findOptions, sort };
    }

    let results;

    if (transaction) {
      const transactionFunction = async (mongoClient, session) => {
        findOptions = { ...findOptions, session };

        return mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .find(findQuery, findOptions).toArray();
      };

      results = await transaction.runWithTransaction(transactionFunction);
    } else {
      results = await this.mongoCollection.find(findQuery, findOptions).toArray();
    }

    return results.map((document) => this.createDppDocument(document));
  }

  /**
   * Store Document entity
   *
   * @param {Document} document
   * @param {MongoDBTransaction} [transaction]
   * @returns {Promise}
   */
  store(document, transaction = undefined) {
    if (
      document.getType() !== this.documentType
      || document.getDataContractId() !== this.contractId
    ) {
      throw new TypeError('Invalid document');
    }

    const filter = { _id: document.getId() };
    const update = { $set: this.createMongoDBDocument(document) };
    let updateOptions = { upsert: true };

    if (transaction) {
      const transactionFunction = async (mongoClient, session) => {
        updateOptions = { ...updateOptions, session };

        return mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .updateOne(
            filter,
            update,
            updateOptions,
          );
      };

      return transaction.runWithTransaction(transactionFunction);
    }

    return this.mongoCollection.updateOne(
      filter,
      update,
      updateOptions,
    );
  }

  /**
   * Delete Document by ID
   *
   * @param {string} id
   * @param {MongoDBTransaction} [transaction]
   * @returns {Promise}
   */
  async delete(id, transaction = undefined) {
    const filter = { _id: id };

    if (transaction) {
      const transactionFunction = async (mongoClient, session) => (
        mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .deleteOne(filter, { session })
      );

      return transaction.runWithTransaction(transactionFunction);
    }

    return this.mongoCollection.deleteOne(filter);
  }

  /**
   * @private
   * @param {Document} document
   * @return {{
   *   _id: string,
   *   ownerId: string,
   *   revision: number,
   *   createdAt?: Date,
   *   updatedAt?: Date,
   *   data: object
   * }}
   */
  createMongoDBDocument(document) {
    const result = {
      _id: document.getId(),
      ownerId: document.getOwnerId(),
      revision: document.getRevision(),
      data: document.getData(),
    };

    const createdAt = document.getCreatedAt();
    if (createdAt) {
      result.createdAt = createdAt;
    }

    const updatedAt = document.getUpdatedAt();
    if (updatedAt) {
      result.updatedAt = updatedAt;
    }

    return result;
  }

  /**
   * @private
   * @param {{
   * _id: string,
   * ownerId: string,
   * revision: number,
   * createdAt?: Date,
   * updatedAt?: Date,
   * data: object}} mongoDBDocument
   * @return {Document}
   */
  createDppDocument({
    _id,
    ownerId,
    revision,
    createdAt,
    updatedAt,
    data,
  }) {
    const rawDocument = {
      $id: _id,
      $type: this.documentType,
      $dataContractId: this.contractId,
      $ownerId: ownerId,
      $revision: revision,
      ...data,
    };

    if (createdAt) {
      rawDocument.$createdAt = createdAt.getTime();
    }

    if (updatedAt) {
      rawDocument.$updatedAt = updatedAt.getTime();
    }

    return new Document(rawDocument);
  }

  /**
   * Create indices for collection
   *
   * @param {Array} indices
   * @returns {Promise<*>}
   */
  async createIndices(indices) {
    return this.mongoCollection.createIndexes(indices);
  }
}

module.exports = DocumentMongoDbRepository;
