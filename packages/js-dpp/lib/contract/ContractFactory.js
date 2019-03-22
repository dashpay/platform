const InvalidContractError = require('./errors/InvalidContractError');

const { decode } = require('../util/serializer');

class ContractFactory {
  /**
   * @param {createContract} createContract
   * @param {validateContract} validateContract
   */
  constructor(createContract, validateContract) {
    this.createContract = createContract;
    this.validateContract = validateContract;
  }

  /**
   * Create Contract
   *
   * @param {string} name
   * @param {Object} documents
   * @return {Contract}
   */
  create(name, documents) {
    return this.createContract({
      name,
      documents,
    });
  }

  /**
   * Create Contract from plain object
   *
   * @param {Object} rawContract
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Contract}
   */
  createFromObject(rawContract, options = { skipValidation: false }) {
    if (!options.skipValidation) {
      const result = this.validateContract(rawContract);

      if (!result.isValid()) {
        throw new InvalidContractError(result.getErrors(), rawContract);
      }
    }

    return this.createContract(rawContract);
  }

  /**
   * Create Contract from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Contract}
   */
  createFromSerialized(payload, options = { skipValidation: false }) {
    const rawContract = decode(payload);

    return this.createFromObject(rawContract, options);
  }
}

module.exports = ContractFactory;
