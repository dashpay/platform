const InvalidDataContractError = require('./errors/InvalidDataContractError');

const DataContract = require('./DataContract');
const generateDataContractId = require('./generateDataContractId');

const DataContractCreateTransition = require('./stateTransition/DataContractCreateTransition');

const SerializedObjectParsingError = require('../errors/SerializedObjectParsingError');

const entropy = require('../util/entropy');

const { decode } = require('../util/serializer');

class DataContractFactory {
  /**
   * @param {validateDataContract} validateDataContract
   */
  constructor(validateDataContract) {
    this.validateDataContract = validateDataContract;
  }

  /**
   * Create Data Contract
   *
   * @param {Buffer} ownerId
   * @param {Object} documents
   * @return {DataContract}
   */
  create(ownerId, documents) {
    const dataContractEntropy = entropy.generate();

    const dataContractId = generateDataContractId(ownerId, dataContractEntropy);

    const dataContract = new DataContract({
      protocolVersion: DataContract.PROTOCOL_VERSION,
      $schema: DataContract.DEFAULTS.SCHEMA,
      $id: dataContractId,
      ownerId,
      documents,
      definitions: {},
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
    try {
      rawDataContract = decode(buffer);
    } catch (error) {
      throw new InvalidDataContractError([
        new SerializedObjectParsingError(
          buffer,
          error,
        ),
      ]);
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
      protocolVersion: DataContract.PROTOCOL_VERSION,
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });
  }
}

module.exports = DataContractFactory;
