const mongo = require('mongodb');

const SVContract = require('./SVContract');
const Reference = require('../revisions/Reference');

const createRevisions = require('../revisions/createRevisions');

class SVContractMongoDbRepository {
  /**
   * @param {Db} mongoDb
   * @param {DashPlatformProtocol} dpp
   */
  constructor(mongoDb, dpp) {
    this.collection = mongoDb.collection('contracts');
    this.dpp = dpp;
  }

  /**
   * Find SV Contract by contractId
   *
   * @param {string} contractId
   * @returns {Promise<SVContract|null>}
   */
  async find(contractId) {
    const result = await this.collection.findOne({
      _id: contractId,
      isDeleted: false,
    });

    if (!result) {
      return null;
    }

    return this.createSVContract(result);
  }

  /**
   * Find list of SV Contract by `reference.stHash`
   *
   * @param {string} hash
   * @returns {Promise<SVContract[]|null>}
   */
  async findAllByReferenceSTHash(hash) {
    const result = await this.collection.find({ 'reference.stHash': hash })
      .toArray();

    return result.map(document => this.createSVContract(document));
  }

  /**
   * Store SV Contract
   *
   * @param {SVContract} svContract
   * @returns {Promise}
   */
  async store(svContract) {
    const rawSVContract = svContract.toJSON();

    rawSVContract.contract = mongo.Binary(
      svContract.getContract().serialize(),
    );

    return this.collection.updateOne(
      { _id: svContract.getContractId() },
      { $set: rawSVContract },
      { upsert: true },
    );
  }

  /**
   * Delete SV Contract
   *
   * @param {SVContract} svContract
   * @returns {Promise}
   */
  async delete(svContract) {
    return this.collection.deleteOne({ _id: svContract.getContractId() });
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
