const enrichDapContractWithBaseDapObject = require('./enrichDapContractWithBaseDapObject');
const validateDapObjectFactory = require('./validateDapObjectFactory');

const DapObjectFactory = require('./DapObjectFactory');

const MissingOptionError = require('../errors/MissingOptionError');

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
   * Create DapObject
   *
   * @param {string} type
   * @param {Object} [data]
   * @return {DapObject}
   */
  create(type, data = {}) {
    return this.getFactory().create(type, data);
  }

  /**
   * Create Dap Object from plain object
   *
   * @param {Object} object
   * @return {DapObject}
   */
  createFromObject(object) {
    return this.getFactory().createFromObject(object);
  }

  /**
   * Create Dap Object from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DapObject}
   */
  createFromSerialized(payload) {
    return this.getFactory().createFromSerialized(payload);
  }

  /**
   *
   * @param {Object|DapObject} dapObject
   * @return {ValidationResult}
   */
  validate(dapObject) {
    return this.validateDapObject(dapObject, this.dap.getDapContract());
  }

  /**
   * @private
   * @return {DapObjectFactory}
   */
  getFactory() {
    if (!this.dap.getUserId()) {
      throw new MissingOptionError('userId');
    }

    if (!this.dap.getDapContract()) {
      throw new MissingOptionError('dapContract');
    }

    this.factory.setUserId(this.dap.getUserId());
    this.factory.setDapContract(this.dap.getDapContract());

    return this.factory;
  }
}

module.exports = DapObjectFacade;
