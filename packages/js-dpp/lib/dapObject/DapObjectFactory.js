const DapObject = require('./DapObject');

const serializer = require('../util/serializer');
const entropy = require('../util/entropy');
const hash = require('../util/hash');

const InvalidDapObjectError = require('./errors/InvalidDapObjectError');

class DapObjectFactory {
  /**
   * @param {DapContract} dapContract
   * @param {string} userId
   * @param {validateDapObject} validateDapObject
   */
  constructor(userId, dapContract, validateDapObject) {
    this.userId = userId;
    this.dapContract = dapContract;
    this.validateDapObject = validateDapObject;
  }

  /**
   * Create DapObject
   *
   * @param {string} type
   * @param {Object} [data]
   * @return {DapObject}
   */
  create(type, data = {}) {
    if (!this.dapContract.isDapObjectDefined(type)) {
      throw Error();
    }

    const rawDapObject = {
      $type: type,
      $scope: hash(this.dapContract.getId() + this.userId),
      $scopeId: entropy.generate(),
      $action: DapObject.DEFAULTS.ACTION,
      $rev: DapObject.DEFAULTS.REVISION,
      ...data,
    };

    return new DapObject(rawDapObject);
  }


  /**
   * Create Dap Object from plain object
   *
   * @param {Object} object
   * @return {DapObject}
   */
  createFromObject(object) {
    const result = this.validateDapObject(object, this.dapContract.getId());

    if (!result.isValid()) {
      throw new InvalidDapObjectError(result.getErrors(), object);
    }

    return new DapObject(object);
  }

  /**
   * Create Dap Object from string/buffer
   *
   * @param {Buffer|string} payload
   * @return {DapObject}
   */
  createFromSerialized(payload) {
    const object = serializer.decode(payload);

    return this.createFromObject(object);
  }

  /**
   * Set User ID
   *
   * @param userId
   * @return {DapObjectFactory}
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
   * Set Dap Contract
   *
   * @param dapContract
   * @return {DapObjectFactory}
   */
  setDapContract(dapContract) {
    this.dapContract = dapContract;

    return this;
  }

  /**
   * Get Dap Contract
   *
   * @return {DapContract}
   */
  getDapContract() {
    return this.dapContract;
  }
}

module.exports = DapObjectFactory;
