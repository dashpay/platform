const DapContractFactory = require('./DapContractFactory');
const validateDapContractFactory = require('./validateDapContractFactory');
const createDapContract = require('./createDapContract');

class DapContractFacade {
  /**
   *
   * @param {JsonSchemaValidator} validator
   */
  constructor(validator) {
    this.validateDapContract = validateDapContractFactory(validator);

    this.factory = new DapContractFactory(
      createDapContract,
      this.validateDapContract,
    );
  }

  /**
   * Create Dap Contract
   *
   * @param {string} name
   * @param {Object} dapObjectsDefinition
   * @return {DapContract}
   */
  create(name, dapObjectsDefinition) {
    return this.factory.create(name, dapObjectsDefinition);
  }

  /**
   * Create Dap Contract from plain object
   *
   * @param {Object} rawDapContract
   * @return {DapContract}
   */
  createFromObject(rawDapContract) {
    return this.factory.createFromObject(rawDapContract);
  }

  /**
   * Create Dap Contract from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DapContract}
   */
  createFromSerialized(payload) {
    return this.factory.createFromSerialized(payload);
  }

  /**
   *
   * @param {DapContract|Object} dapContract
   * @return {ValidationResult}
   */
  validate(dapContract) {
    return this.validateDapContract(dapContract);
  }
}

module.exports = DapContractFacade;
