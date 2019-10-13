const ContractFactory = require('./ContractFactory');
const validateContractFactory = require('./validateContractFactory');
const createContract = require('./createContract');

class ContractFacade {
  /**
   * @param {JsonSchemaValidator} validator
   */
  constructor(validator) {
    this.validateContract = validateContractFactory(validator);

    this.factory = new ContractFactory(
      createContract,
      this.validateContract,
    );
  }

  /**
   * Create Contract
   *
   * @param {string} contractId
   * @param {Object} documents
   * @return {Contract}
   */
  create(contractId, documents) {
    return this.factory.create(contractId, documents);
  }

  /**
   * Create Contract from plain object
   *
   * @param {RawContract} rawContract
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Contract}
   */
  createFromObject(rawContract, options = { skipValidation: false }) {
    return this.factory.createFromObject(rawContract, options);
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
    return this.factory.createFromSerialized(payload, options);
  }

  /**
   * Validate Contract
   *
   * @param {Contract|RawContract} contract
   * @return {ValidationResult}
   */
  validate(contract) {
    return this.validateContract(contract);
  }
}

module.exports = ContractFacade;
