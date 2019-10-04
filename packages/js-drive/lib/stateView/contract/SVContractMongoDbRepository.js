const mongo = require('mongodb');

const SVContract = require('./SVContract');
const Reference = require('../revisions/Reference');

const createRevisions = require('../revisions/createRevisions');

class SVContractMongoDbRepository {
  /**
   * @param {Db} mongoDatabase
   * @param {DashPlatformProtocol} dpp
   */
  constructor(mongoDatabase, dpp) {
    this.collectionName = this.getCollectionName();
    this.mongoCollection = mongoDatabase.collection(this.collectionName);
    this.dpp = dpp;
    this.databaseName = mongoDatabase.databaseName;
    this.mongoDatabase = mongoDatabase;
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
    return 'contracts';
  }

  /**
   * Find SV Contract by contractId
   *
   * @param {string} contractId
   * @param {MongoDBTransaction} [transaction]
   * @returns {Promise<SVContract|null>}
   */
  async find(contractId, transaction = undefined) {
    const findQuery = {
      _id: contractId,
      isDeleted: false,
    };

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

    return this.createSVContract(result);
  }

  /**
   * Find list of SV Contract by `reference.stHash`
   *
   * @param {string} hash
   * @param {MongoDBTransaction} [transaction]
   * @returns {Promise<SVContract[]|null>}
   */
  async findAllByReferenceSTHash(hash, transaction = undefined) {
    const findQuery = { 'reference.stHash': hash };

    let result;
    if (transaction) {
      const transactionFunction = async (mongoClient, session) => (
        mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .find(findQuery, { session })
          .toArray()
      );

      result = await transaction.runWithTransaction(transactionFunction);
    } else {
      result = await this.mongoCollection.find(findQuery)
        .toArray();
    }

    return result.map(document => this.createSVContract(document));
  }

  /**
   * Store SV Contract
   *
   * @param {SVContract} svContract
   * @param {MongoDBTransaction} [transaction]
   * @returns {Promise}
   */
  async store(svContract, transaction = undefined) {
    const rawSVContract = svContract.toJSON();
    rawSVContract.contract = mongo.Binary(
      svContract.getContract().serialize(),
    );

    const filter = { _id: svContract.getContractId() };
    const update = { $set: rawSVContract };
    let updateOptions = { upsert: true };

    if (transaction) {
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

      return transaction.runWithTransaction(transactionFunction);
    }

    return this.mongoCollection.updateOne(
      filter,
      update,
      updateOptions,
    );
  }

  /**
   * Delete SV Contract
   *
   * @param {SVContract} svContract
   * @param {MongoDBTransaction} [transaction]
   * @returns {Promise}
   */
  async delete(svContract, transaction = undefined) {
    const filter = { _id: svContract.getContractId() };
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
   * @typedef createSVContract
   * @param {Object} rawSVContract
   * @returns {SVContract}
   */
  createSVContract({
    contractId,
    userId,
    contract: serializedRawContract,
    reference,
    isDeleted,
    previousRevisions,
  }) {
    const contract = this.dpp.contract.createFromSerialized(
      serializedRawContract.buffer,
      { skipValidation: true },
    );

    return new SVContract(
      contractId,
      userId,
      contract,
      new Reference(reference),
      isDeleted,
      createRevisions(previousRevisions),
    );
  }
}

module.exports = SVContractMongoDbRepository;
