const InvalidDapContractStructureError = require('./errors/InvalidDapContractStructureError');
const DapContract = require('./DapContract');

const serializer = require('../util/serializer');

class DapContractFactory {
  /**
   * @param {validateDapContract} validateDapContract
   */
  constructor(validateDapContract) {
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
    return new DapContract(name, dapObjectsDefinition);
  }

  /**
   * Create Dap Contract from plain object
   *
   * @param {Object} object
   * @return {DapContract}
   */
  createFromObject(object) {
    const errors = this.validateDapContract(object);

    if (errors.length) {
      throw new InvalidDapContractStructureError(errors, object);
    }

    const dapContract = this.create(object.name, object.dapObjectsDefinition);

    dapContract.setSchema(object.$schema);
    dapContract.setVersion(object.version);

    if (object.definitions) {
      dapContract.setDefinitions(object.definitions);
    }

    return dapContract;
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
