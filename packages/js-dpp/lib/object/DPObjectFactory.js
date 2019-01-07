const DPObject = require('./DPObject');

const { decode } = require('../util/serializer');
const entropy = require('../util/entropy');
const hash = require('../util/hash');

const InvalidDPObjectError = require('./errors/InvalidDPObjectError');
const InvalidDPObjectTypeError = require('../errors/InvalidDPObjectTypeError');

class DPObjectFactory {
  /**
   * @param {DPContract} dpContract
   * @param {string} userId
   * @param {validateDPObject} validateDPObject
   */
  constructor(userId, dpContract, validateDPObject) {
    this.userId = userId;
    this.dpContract = dpContract;
    this.validateDPObject = validateDPObject;
  }

  /**
   * Create DPObject
   *
   * @param {string} type
   * @param {Object} [data]
   * @return {DPObject}
   */
  create(type, data = {}) {
    if (!this.dpContract.isDPObjectDefined(type)) {
      throw new InvalidDPObjectTypeError(type, this.dpContract);
    }

    const rawDPObject = {
      $type: type,
      $scope: hash(this.dpContract.getId() + this.userId),
      $scopeId: entropy.generate(),
      $action: DPObject.DEFAULTS.ACTION,
      $rev: DPObject.DEFAULTS.REVISION,
      ...data,
    };

    return new DPObject(rawDPObject);
  }


  /**
   * Create DP Object from plain object
   *
   * @param {Object} rawDPObject
   * @return {DPObject}
   */
  createFromObject(rawDPObject) {
    const result = this.validateDPObject(rawDPObject, this.dpContract);

    if (!result.isValid()) {
      throw new InvalidDPObjectError(result.getErrors(), rawDPObject);
    }

    return new DPObject(rawDPObject);
  }

  /**
   * Create DPObject from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DPObject}
   */
  createFromSerialized(payload) {
    const rawDPObject = decode(payload);

    return this.createFromObject(rawDPObject);
  }

  /**
   * Set User ID
   *
   * @param userId
   * @return {DPObjectFactory}
   */
  setUserId(userId) {
    this.userId = userId;

    return this;
  }

  /**
   * Get User ID
   *
   * @return {string}
   */
  getUserId() {
    return this.userId;
  }

  /**
   * Set DP Contract
   *
   * @param {DPContract} dpContract
   * @return {DPObjectFactory}
   */
  setDPContract(dpContract) {
    this.dpContract = dpContract;

    return this;
  }

  /**
   * Get DP Contract
   *
   * @return {DPContract}
   */
  getDPContract() {
    return this.dpContract;
  }
}

module.exports = DPObjectFactory;
