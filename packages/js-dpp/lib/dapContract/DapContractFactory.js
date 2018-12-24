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
    const object = decode(payload);

    return this.createFromObject(object);
  }
}

module.exports = DapContractFactory;
