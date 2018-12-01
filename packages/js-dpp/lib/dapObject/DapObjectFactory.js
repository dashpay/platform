const DapObject = require('./DapObject');

const InvalidDapObjectStructureError = require('./errors/InvalidDapObjectStructureError');

const serializer = require('../util/serializer');

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
    if (this.dapContract.isDapObjectDefined(type)) {
      throw Error();
    }

    return new DapObject(this.dapContract, this.userId, type, data);
  }


  /**
   * Create Dap Object from plain object
   *
   * @param {Object} object
   * @return {DapObject}
   */
  createFromObject(object) {
    const errors = this.validateDapObject(object, this.dapContract.getId());

    if (errors.length) {
      throw new InvalidDapObjectStructureError(errors, object);
    }

    return this.create(object.$type, object);
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
