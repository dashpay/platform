const Identifier = require('@dashevo/dpp/lib/Identifier');

const lodashSet = require('lodash.set');
const lodashGet = require('lodash.get');

const convertFieldName = require('./convertFieldName');

const InvalidQueryError = require('../errors/InvalidQueryError');
const getIndexedFieldsFromDocumentSchema = require('../query/getIndexedFieldsFromDocumentSchema');

class DocumentMongoDbRepository {
  /**
   * @param {Db} mongoDatabase
   * @param {convertWhereToMongoDbQuery} convertWhereToMongoDbQuery
   * @param {validateQuery} validateQuery
   * @param {DataContract} dataContract
   * @param {string} documentType
   */
  constructor(
    mongoDatabase,
    convertWhereToMongoDbQuery,
    validateQuery,
    dataContract,
    documentType,
  ) {
    this.mongoDatabase = mongoDatabase;
    this.convertWhereToMongoDbQuery = convertWhereToMongoDbQuery;
    this.validateQuery = validateQuery;
    this.documentType = documentType;
    this.dataContract = dataContract;
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
   * Find document IDs by query
   *
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   * @param {MongoDBTransaction} [transaction]
   *
   * @returns {Promise<Identifier[]>}
   * @throws {InvalidQueryError}
   */
  async find(query = {}, transaction = undefined) {
    const documentSchema = this.dataContract.getDocumentSchema(this.documentType);

    const result = this.validateQuery(query, documentSchema);

    if (!result.isValid()) {
      throw new InvalidQueryError(result.getErrors());
    }

    let findQuery = {};
    let findOptions = {
      promoteBuffers: true, // Automatically convert Blob to Buffer
      projection: { _id: 1 },
    };

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

    // eslint-disable-next-line no-underscore-dangle
    return results.map((mongoDbDocument) => new Identifier(mongoDbDocument._id));
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
      || !document.getDataContractId().equals(this.dataContract.getId())
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
   *   _id: Buffer,
   *   ownerId: Buffer,
   *   revision: number,
   *   createdAt?: Date,
   *   updatedAt?: Date,
   *   data: Object
   * }}
   */
  createMongoDBDocument(document) {
    const documentSchema = this.dataContract.getDocumentSchema(document.getType());
    const documentIndexedProperties = getIndexedFieldsFromDocumentSchema(documentSchema)
      .map((properties) => properties.map((property) => Object.keys(property)[0])).flat();

    const uniqueDocumentIndexedDataProperties = [...new Set(documentIndexedProperties)]
      .filter((field) => !field.startsWith('$'));

    const rawDocument = document.toObject();

    const data = uniqueDocumentIndexedDataProperties.reduce((indexedProperties, propertyPath) => {
      const propertyValue = lodashGet(rawDocument, propertyPath);

      return propertyValue === undefined ? indexedProperties : lodashSet(
        indexedProperties,
        propertyPath,
        lodashGet(rawDocument, propertyPath),
      );
    }, {});

    const result = {
      _id: document.getId(),
      ownerId: document.getOwnerId(),
      revision: document.getRevision(),
      protocolVersion: document.getProtocolVersion(),
      data,
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
   * Create indices for collection
   *
   * @param {Array} indices
   * @returns {Promise<*>}
   */
  async createIndices(indices) {
    const modifiedIndices = indices.map((index) => ({
      ...index,
      ...Object.keys(index.key)
        .reduce((keyObject, key) => ({
          ...keyObject,
          partialFilterExpression: {
            ...keyObject.partialFilterExpression,
            [key]: { $exists: true },
          },
        }), { partialFilterExpression: {} }),
    }));

    return this.mongoCollection.createIndexes(modifiedIndices);
  }
}

module.exports = DocumentMongoDbRepository;
