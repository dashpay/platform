const InvalidDataContractError = require('./errors/InvalidDataContractError');

const DataContract = require('./DataContract');
const generateDataContractId = require('./generateDataContractId');

const DataContractCreateTransition = require('./stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const generateEntropy = require('../util/generateEntropy');
const ConsensusError = require('../errors/consensus/ConsensusError');

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
    const dataContractEntropy = generateEntropy();

    const dataContractId = generateDataContractId(ownerId, dataContractEntropy);

    const dataContract = new DataContract({
      protocolVersion: this.dpp.getProtocolVersion(),
      $schema: DataContract.DEFAULTS.SCHEMA,
      $id: dataContractId,
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
        this.dpp.getProtocolVersion(),
      );

      rawDataContract.protocolVersion = protocolVersion;
    } catch (error) {
      if (error instanceof ConsensusError) {
        throw new InvalidDataContractError([error]);
      }

      throw error;
    }

    return this.createFromObject(rawDataContract, options);
  }

  /**
   * Create Data Contract State Transition
   *
   * @param {DataContract} dataContract
   * @return {DataContractCreateTransition}
   */
  createStateTransition(dataContract) {
    return new DataContractCreateTransition({
      protocolVersion: this.dpp.getProtocolVersion(),
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });
  }
}

module.exports = DataContractFactory;
