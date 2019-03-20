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
   * @param {Object} documents
   * @return {DPContract}
   */
  create(name, documents) {
    return this.createDPContract({
      name,
      documents,
    });
  }

  /**
   * Create DP Contract from plain object
   *
   * @param {Object} rawDPContract
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DPContract}
   */
  createFromObject(rawDPContract, options = { skipValidation: false }) {
    if (!options.skipValidation) {
      const result = this.validateDPContract(rawDPContract);

      if (!result.isValid()) {
        throw new InvalidDPContractError(result.getErrors(), rawDPContract);
      }
    }

    return this.createDPContract(rawDPContract);
  }

  /**
   * Create DP Contract from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DPContract}
   */
  createFromSerialized(payload, options = { skipValidation: false }) {
    const rawDPContract = decode(payload);

    return this.createFromObject(rawDPContract, options);
  }
}

module.exports = DPContractFactory;
