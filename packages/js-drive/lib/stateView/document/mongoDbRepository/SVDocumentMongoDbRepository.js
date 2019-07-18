const Document = require('@dashevo/dpp/lib/document/Document');

const SVDocument = require('../SVDocument');
const Reference = require('../../revisions/Reference');

const convertFieldName = require('./convertFieldName');

const createRevisions = require('../../revisions/createRevisions');

const InvalidQueryError = require('../errors/InvalidQueryError');

class SVDocumentMongoDbRepository {
  /**
   * @param {Db} mongoClient
   * @param {sanitizer} sanitizer
   * @param {convertWhereToMongoDbQuery} convertWhereToMongoDbQuery
   * @param {validateQuery} validateQuery
   * @param {string} documentType
   */
  constructor(mongoClient, sanitizer, convertWhereToMongoDbQuery, validateQuery, documentType) {
    this.mongoClient = mongoClient.collection(`documents_${documentType}`);
    this.sanitizer = sanitizer;
    this.convertWhereToMongoDbQuery = convertWhereToMongoDbQuery;
    this.validateQuery = validateQuery;
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

    const results = await this.mongoClient.find(findQuery, findOptions).toArray();

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

    rawDocument.$meta = {
      userId,
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
