const InvalidDapContractError = require('./errors/InvalidDapContractError');

const serializer = require('../util/serializer');

class DapContractFactory {
  /**
   * @param {validateDapContract} validateDapContract
   * @param {createDapContract} createDapContract
   */
  constructor(validateDapContract, createDapContract) {
    this.validateDapContract = validateDapContract;
    this.createDapContract = createDapContract;
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
   * @param {Object} object
   * @return {DapContract}
   */
  createFromObject(object) {
    const result = this.validateDapContract(object);

    if (!result.isValid()) {
      throw new InvalidDapContractError(result.getErrors(), object);
    }

    return this.createDapContract(object);
  }

  /**
   * Create Dap Contract from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DapContract}
   */
  createFromSerialized(payload) {
    const object = serializer.encode(payload);

    return this.createFromObject(object);
  }
}

module.exports = DapContractFactory;
