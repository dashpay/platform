const mongo = require('mongodb');
const Reference = require('../Reference');
const DapContract = require('./DapContract');

class DapContractMongoDbRepository {
  /**
   * @param {Db} mongoDb
   * @param {serializer} serializer
   */
  constructor(mongoDb, { encode, decode }) {
    this.collection = mongoDb.collection('dapContracts');
    this.encode = encode;
    this.decode = decode;
  }

  /**
   * Find DapContract by dapId
   *
   * @param {string} dapId
   * @returns {Promise<DapContract|null>}
   */
  async find(dapId) {
    const result = await this.collection.findOne({ _id: dapId, isDeleted: false });
    if (!result) {
      return null;
    }
    return this.toDapContract(result);
  }

  /**
   * Find list of DapContract by `reference.stHeaderHash`
   *
   * @param {string} hash
   * @returns {Promise<[DapContract]|null>}
   */
  async findAllByReferenceSTHeaderHash(hash) {
    const result = await this.collection.find({ 'reference.stHeaderHash': hash })
      .toArray();

    return result.map(document => this.toDapContract(document));
  }

  /**
   * Store DapContract entity
   *
   * @param {DapContract} dapContract
   * @returns {Promise}
   */
  async store(dapContract) {
    const dapContractData = dapContract.toJSON();

    dapContractData.data = mongo.Binary(
      this.encode(dapContractData.data),
    );

    return this.collection.updateOne(
      { _id: dapContractData.dapId },
      { $set: dapContractData },
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
   * @param {object} contractFromDb
   * @returns {DapContract}
   */
  toDapContract(contractFromDb) {
    const contractData = Object.assign({}, contractFromDb, {
      data: this.decode(contractFromDb.data.buffer),
    });

    const data = {
      dapname: contractData.dapName,
      dapver: contractData.version,
      ...contractData.data,
    };

    const {
      dapId,
      isDeleted,
      reference: referenceData,
      previousVersions: previousVersionsData,
    } = contractData;

    const reference = new Reference(
      referenceData.blockHash,
      referenceData.blockHeight,
      referenceData.stHeaderHash,
      referenceData.stPacketHash,
      referenceData.objectHash,
    );

    return new DapContract(
      dapId,
      data,
      reference,
      isDeleted,
      this.toPreviousVersions(
        previousVersionsData,
      ),
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
