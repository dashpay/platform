const InvalidDataContractError = require('./errors/InvalidDataContractError');

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
  createFromObject(rawDataContract, options = { skipValidation: false }) {
    if (!options.skipValidation) {
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
  createFromSerialized(payload, options = { skipValidation: false }) {
    const rawDataContract = decode(payload);

    return this.createFromObject(rawDataContract, options);
  }
}

module.exports = DataContractFactory;
