const enrichDPContractWithBaseDPObject = require('./enrichDPContractWithBaseDPObject');
const validateDPObjectFactory = require('./validateDPObjectFactory');

const DPObjectFactory = require('./DPObjectFactory');

const MissingOptionError = require('../errors/MissingOptionError');

class DPObjectFacade {
  /**
   *
   * @param {DashPlatformProtocol} dpp
   * @param {JsonSchemaValidator} validator
   */
  constructor(dpp, validator) {
    this.dpp = dpp;

    this.validateDPObject = validateDPObjectFactory(
      validator,
      enrichDPContractWithBaseDPObject,
    );

    this.factory = new DPObjectFactory(
      dpp.getUserId(),
      dpp.getDPContract(),
      this.validateDPObject,
    );
  }

  /**
   * Create DPObject
   *
   * @param {string} type
   * @param {Object} [data]
   * @return {DPObject}
   */
  create(type, data = {}) {
    return this.getFactory().create(type, data);
  }

  /**
   * Create DPObject from plain object
   *
   * @param {Object} rawDPObject
   * @return {DPObject}
   */
  createFromObject(rawDPObject) {
    return this.getFactory().createFromObject(rawDPObject);
  }

  /**
   * Create DPObject from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DPObject}
   */
  createFromSerialized(payload) {
    return this.getFactory().createFromSerialized(payload);
  }

  /**
   *
   * @param {Object|DPObject} dpObject
   * @return {ValidationResult}
   */
  validate(dpObject) {
    return this.validateDPObject(dpObject, this.dpp.getDPContract());
  }

  /**
   * @private
   * @return {DPObjectFactory}
   */
  getFactory() {
    if (!this.dpp.getUserId()) {
      throw new MissingOptionError(
        'userId',
        'Can\'t create packet because User ID is not set, use setUserId method',
      );
    }

    if (!this.dpp.getDPContract()) {
      throw new MissingOptionError(
        'dpContract',
        'Can\'t create DP Object because DP Contract is not set, use setDPContract method',
      );
    }

    this.factory.setUserId(this.dpp.getUserId());
    this.factory.setDPContract(this.dpp.getDPContract());

    return this.factory;
  }
}

module.exports = DPObjectFacade;
