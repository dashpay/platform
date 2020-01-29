const InvalidDataContractError = require('./errors/InvalidDataContractError');

const DataContractStateTransition = require('./stateTransition/DataContractStateTransition');

const { decode } = require('../util/serializer');

class DataContractFactory {
  /**
   * @param {createDataContract} createDataContract
   * @param {validateDataContract} validateDataContract
   */
  constructor(createDataContract, validateDataContract) {
    this.createDataContract = createDataContract;
    this.validateDataContract = validateDataContract;
  }

  /**
   * Create Data Contract
   *
   * @param {string} contractId
   * @param {Object} documents
   * @return {DataContract}
   */
  create(contractId, documents) {
    return this.createDataContract({
      contractId,
      documents,
    });
  }

  /**
   * Create Data Contract from plain object
   *
   * @param {RawDataContract} rawDataContract
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContract}
   */
  createFromObject(rawDataContract, options = { }) {
    const opts = { skipValidation: false, ...options };

    if (!opts.skipValidation) {
      const result = this.validateDataContract(rawDataContract);

      if (!result.isValid()) {
        throw new InvalidDataContractError(result.getErrors(), rawDataContract);
      }
    }

    return this.createDataContract(rawDataContract);
  }

  /**
   * Create Data Contract from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContract}
   */
  createFromSerialized(payload, options = { }) {
    const rawDataContract = decode(payload);

    return this.createFromObject(rawDataContract, options);
  }

  /**
   * Create Data Contract State Transition
   *
   * @param {DataContract} dataContract
   * @return {DataContractStateTransition}
   */
  createStateTransition(dataContract) {
    return new DataContractStateTransition(dataContract);
  }
}

module.exports = DataContractFactory;
