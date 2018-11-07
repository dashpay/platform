const Reference = require('../Reference');
const DapContract = require('./DapContract');

class DapContractMongoDbRepository {
  /**
   * @param {Db} mongoDb
   * @param {sanitizeData} sanitizeData
   */
  constructor(mongoDb, { sanitize, unsanitize }) {
    this.collection = mongoDb.collection('dapContracts');
    this.sanitize = sanitize;
    this.unsanitize = unsanitize;
  }

  /**
   * Find DapContract by dapId
   *
   * @param {string} dapId
   * @returns {Promise<DapContract|null>}
   */
  async find(dapId) {
    const result = await this.collection.findOne({ _id: dapId });

    if (!result) {
      return null;
    }

    const dapContractData = this.unsanitize(result);

    const previousVersions = this.toPreviousVersions(dapContractData.previousVersions);
    return this.toDapContract(dapContractData, dapContractData.reference, previousVersions);
  }

  /**
   * Store DapContract entity
   *
   * @param {DapContract} dapContract
   * @returns {Promise}
   */
  async store(dapContract) {
    const dapContractData = dapContract.toJSON();

    return this.collection.updateOne(
      { _id: dapContractData.dapId },
      { $set: this.sanitize(dapContractData) },
      { upsert: true },
    );
  }

  /**
   * Delete DapContract entity
   *
   * @param {DapContract} dapContract
   * @returns {Promise}
   */
  async delete(dapContract) {
    return this.collection.deleteOne({ _id: dapContract.dapId });
  }

  /**
   * @private
   * @param {object} dapContractData
   * @param {object} referenceData
   * @param {array} previousVersions
   * @returns {DapContract}
   */
  toDapContract(dapContractData = {}, referenceData = {}, previousVersions = []) {
    const reference = new Reference(
      referenceData.blockHash,
      referenceData.blockHeight,
      referenceData.stHeaderHash,
      referenceData.stPacketHash,
      referenceData.objectHash,
    );
    return new DapContract(
      dapContractData.dapId,
      dapContractData.dapName,
      reference,
      dapContractData.schema,
      dapContractData.version,
      previousVersions,
    );
  }

  /**
   * @private
   * @param {array} previousVersionsData
   * @returns {{version: number, reference: Reference}[]}
   */
  toPreviousVersions(previousVersionsData = []) {
    return previousVersionsData.map((previousRevision) => {
      const previousVersion = previousRevision.version;
      const previousReferenceData = previousRevision.reference;
      return {
        version: previousVersion,
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
}

module.exports = DapContractMongoDbRepository;
