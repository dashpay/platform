const InvalidDapContractError = require('./errors/InvalidDapContractError');

const { decode } = require('../util/serializer');

class DapContractFactory {
  /**
   * @param {createDapContract} createDapContract
   * @param {validateDapContract} validateDapContract
   */
  constructor(createDapContract, validateDapContract) {
    this.createDapContract = createDapContract;
    this.validateDapContract = validateDapContract;
  }

  /**
   * Create Dap Contract
   *
   * @param {string} name
   * @param {Object} dapObjectsDefinition
   * @return {DapContract}
   */
  create(name, dapObjectsDefinition) {
    return this.createDapContract({
      name,
      dapObjectsDefinition,
    });
  }

  /**
   * Create Dap Contract from plain object
   *
   * @param {Object} rawDapContract
   * @return {DapContract}
   */
  createFromObject(rawDapContract) {
    const result = this.validateDapContract(rawDapContract);

    if (!result.isValid()) {
      throw new InvalidDapContractError(result.getErrors(), rawDapContract);
    }

    return this.createDapContract(rawDapContract);
  }

  /**
   * Create Dap Contract from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DapContract}
   */
  createFromSerialized(payload) {
    const rawDapContract = decode(payload);

    return this.createFromObject(rawDapContract);
  }
}

module.exports = DapContractFactory;
