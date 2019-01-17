const DPContractFactory = require('./DPContractFactory');
const validateDPContractFactory = require('./validateDPContractFactory');
const createDPContract = require('./createDPContract');

class DPContractFacade {
  /**
   *
   * @param {JsonSchemaValidator} validator
   */
  constructor(validator) {
    this.validateDPContract = validateDPContractFactory(validator);

    this.factory = new DPContractFactory(
      createDPContract,
      this.validateDPContract,
    );
  }

  /**
   * Create DP Contract
   *
   * @param {string} name
   * @param {Object} dpObjectsDefinition
   * @return {DPContract}
   */
  create(name, dpObjectsDefinition) {
    return this.factory.create(name, dpObjectsDefinition);
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
    return this.factory.createFromObject(rawDPContract, options);
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
    return this.factory.createFromSerialized(payload, options);
  }

  /**
   *
   * @param {DPContract|Object} dpContract
   * @return {ValidationResult}
   */
  validate(dpContract) {
    return this.validateDPContract(dpContract);
  }
}

module.exports = DPContractFacade;
