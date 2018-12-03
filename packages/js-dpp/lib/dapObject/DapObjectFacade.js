const enrichDapContractWithBaseDapObject = require('./enrichDapContractWithBaseDapObject');
const validateDapObjectFactory = require('./validateDapObjectFactory');

const DapObjectFactory = require('./DapObjectFactory');

class DapObjectFacade {
  /**
   *
   * @param {DashApplicationProtocol} dap
   * @param {JsonSchemaValidator} validator
   */
  constructor(dap, validator) {
    this.dap = dap;

    this.validateDapObject = validateDapObjectFactory(
      validator,
      enrichDapContractWithBaseDapObject,
    );

    this.factory = new DapObjectFactory(
      dap.getUserId(),
      dap.getDapContract(),
      this.validateDapObject,
    );
  }

  /**
   * Update dependencies
   *
   * @param {DashApplicationProtocol} dap
   */
  updateDependencies(dap) {
    this.factory.setUserId(dap.getUserId());
    this.factory.setDapContract(dap.getDapContract());
  }

  /**
   * Create DapObject
   *
   * @param {string} type
   * @param {Object} [data]
   * @return {DapObject}
   */
  create(type, data = {}) {
    return this.factory.create(type, data);
  }

  /**
   * Create Dap Object from plain object
   *
   * @param {Object} object
   * @return {DapObject}
   */
  createFromObject(object) {
    return this.factory.createFromObject(object);
  }

  /**
   * Create Dap Object from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DapObject}
   */
  createFromSerialized(payload) {
    return this.factory.createFromSerialized(payload);
  }

  /**
   *
   * @param {Object|DapObject} dapObject
   * @return {Object[]|*}
   */
  validate(dapObject) {
    return this.validateDapObject(dapObject, this.dap.getDapContract());
  }
}

module.exports = DapObjectFacade;
