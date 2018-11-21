

const DapObject = require('./DapObject');
const InvalidDapObjectStructureError = require('./errors/InvalidDapObjectStructureError');

const serializer = require('../util/serializer');

class DapObjectFactory {
  /**
   * @param {DapContract} dapContract
   * @param {string} blockchainUserId
   * @param {validateDapObject} validateDapObject
   */
  constructor(dapContract, blockchainUserId, validateDapObject) {
    this.dapContract = dapContract;
    this.blockchainUserId = blockchainUserId;
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

    return new DapObject(this.dapContract, this.blockchainUserId, type, data);
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
}

module.exports = DapObjectFactory;
