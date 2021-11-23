const InvalidDataContractError = require('./errors/InvalidDataContractError');

const DataContract = require('./DataContract');
const generateDataContractId = require('./generateDataContractId');

const DataContractCreateTransition = require('./stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const entropyGenerator = require('../util/entropyGenerator');
const AbstractConsensusError = require('../errors/consensus/AbstractConsensusError');
const DataContractUpdateTransition = require('./stateTransition/DataContractUpdateTransition/DataContractUpdateTransition');

class DataContractFactory {
  /**
   * @param {DashPlatformProtocol} dpp
   * @param {validateDataContract} validateDataContract
   * @param {decodeProtocolEntity} decodeProtocolEntity
   */
  constructor(dpp, validateDataContract, decodeProtocolEntity) {
    this.dpp = dpp;
    this.validateDataContract = validateDataContract;
    this.decodeProtocolEntity = decodeProtocolEntity;
  }

  /**
   * Create Data Contract
   *
   * @param {Buffer} ownerId
   * @param {Object} documents
   * @return {DataContract}
   */
  create(ownerId, documents) {
    const { generate } = entropyGenerator;
    const dataContractEntropy = generate();

    const dataContractId = generateDataContractId(ownerId, dataContractEntropy);

    const dataContract = new DataContract({
      protocolVersion: this.dpp.getProtocolVersion(),
      $schema: DataContract.DEFAULTS.SCHEMA,
      $id: dataContractId,
      $version: 1,
      ownerId,
      documents,
      $defs: {},
    });

    dataContract.setEntropy(dataContractEntropy);

    return dataContract;
  }

  /**
   * Create Data Contract from plain object
   *
   * @param {RawDataContract} rawDataContract
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<DataContract>}
   */
  async createFromObject(rawDataContract, options = { }) {
    const opts = { skipValidation: false, ...options };

    if (!opts.skipValidation) {
      const result = await this.validateDataContract(rawDataContract);

      if (!result.isValid()) {
        throw new InvalidDataContractError(result.getErrors(), rawDataContract);
      }
    }

    return new DataContract(rawDataContract);
  }

  /**
   * Create Data Contract from buffer
   *
   * @param {Buffer} buffer
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<DataContract>}
   */
  async createFromBuffer(buffer, options = { }) {
    let rawDataContract;
    let protocolVersion;

    try {
      [protocolVersion, rawDataContract] = this.decodeProtocolEntity(
        buffer,
      );

      rawDataContract.protocolVersion = protocolVersion;
    } catch (error) {
      if (error instanceof AbstractConsensusError) {
        throw new InvalidDataContractError([error]);
      }

      throw error;
    }

    return this.createFromObject(rawDataContract, options);
  }

  /**
   * Create Data Contract Create State Transition
   *
   * @param {DataContract} dataContract
   * @return {DataContractCreateTransition}
   */
  createDataContractCreateTransition(dataContract) {
    return new DataContractCreateTransition({
      protocolVersion: this.dpp.getProtocolVersion(),
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });
  }

  /**
   * Create Data Contract Update State Transition
   *
   * @param {DataContract} dataContract
   * @return {DataContractUpdateTransition}
   */
  createDataContractUpdateTransition(dataContract) {
    return new DataContractUpdateTransition({
      protocolVersion: this.dpp.getProtocolVersion(),
      dataContract: dataContract.toObject(),
    });
  }
}

module.exports = DataContractFactory;
