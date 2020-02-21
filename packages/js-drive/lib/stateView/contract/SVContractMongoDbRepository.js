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
   * @param {MongoDBTransaction} [stateViewTransaction]
   * @returns {Promise<SVContract|null>}
   */
  async find(contractId, stateViewTransaction = undefined) {
    const findQuery = {
      _id: contractId,
      isDeleted: false,
    };

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

    return this.createSVContract(result);
  }

  /**
   * Find list of SV Contract by `reference.stHash`
   *
   * @param {string} hash
   * @param {MongoDBTransaction} [stateViewTransaction]
   * @returns {Promise<SVContract[]|null>}
   */
  async findAllByReferenceSTHash(hash, stateViewTransaction = undefined) {
    const findQuery = { 'reference.stHash': hash };

    let result;
    if (stateViewTransaction) {
      const transactionFunction = async (mongoClient, session) => (
        mongoClient
          .db(this.databaseName)
          .collection(this.collectionName)
          .find(findQuery, { session })
          .toArray()
      );

      result = await stateViewTransaction.runWithTransaction(transactionFunction);
    } else {
      result = await this.mongoCollection.find(findQuery)
        .toArray();
    }

    return Promise.all(
      result.map(document => this.createSVContract(document)),
    );
  }

  /**
   * Store SV Contract
   *
   * @param {SVContract} svContract
   * @param {MongoDBTransaction} [stateViewTransaction]
   * @returns {Promise}
   */
  async store(svContract, stateViewTransaction = undefined) {
    const rawSVContract = svContract.toJSON();
    rawSVContract.contract = mongo.Binary(
      svContract.getDataContract().serialize(),
    );

    const filter = { _id: svContract.getId() };
    const update = { $set: rawSVContract };
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
   * Delete SV Contract
   *
   * @param {SVContract} svContract
   * @param {MongoDBTransaction} [stateViewTransaction]
   * @returns {Promise}
   */
  async delete(svContract, stateViewTransaction = undefined) {
    const filter = { _id: svContract.getId() };
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
   * @typedef createSVContract
   * @param {Object} rawSVContract
   * @returns {Promise<SVContract>}
   */
  async createSVContract({
    contract: serializedRawContract,
    reference,
    isDeleted,
    previousRevisions,
  }) {
    const contract = await this.dpp.dataContract.createFromSerialized(
      serializedRawContract.buffer,
      { skipValidation: true },
    );

    return new SVContract(
      contract,
      new Reference(reference),
      isDeleted,
      createRevisions(previousRevisions),
    );
  }
}

module.exports = SVContractMongoDbRepository;
