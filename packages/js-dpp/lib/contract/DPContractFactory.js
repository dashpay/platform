const InvalidDPContractError = require('./errors/InvalidDPContractError');

const { decode } = require('../util/serializer');

class DPContractFactory {
  /**
   * @param {createDPContract} createDPContract
   * @param {validateDPContract} validateDPContract
   */
  constructor(createDPContract, validateDPContract) {
    this.createDPContract = createDPContract;
    this.validateDPContract = validateDPContract;
  }

  /**
   * Create DP Contract
   *
   * @param {string} name
   * @param {Object} dpObjectsDefinition
   * @return {DPContract}
   */
  create(name, dpObjectsDefinition) {
    return this.createDPContract({
      name,
      dpObjectsDefinition,
    });
  }

  /**
   * Create DP Contract from plain object
   *
   * @param {Object} rawDPContract
   * @return {DPContract}
   */
  createFromObject(rawDPContract) {
    const result = this.validateDPContract(rawDPContract);

    if (!result.isValid()) {
      throw new InvalidDPContractError(result.getErrors(), rawDPContract);
    }

    return this.createDPContract(rawDPContract);
  }

  /**
   * Create DP Contract from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DPContract}
   */
  createFromSerialized(payload) {
    const rawDPContract = decode(payload);

    return this.createFromObject(rawDPContract);
  }
}

module.exports = DPContractFactory;
