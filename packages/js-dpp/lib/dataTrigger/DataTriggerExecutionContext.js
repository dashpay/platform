const EncodedBuffer = require('../util/encoding/EncodedBuffer');

class DataTriggerExecutionContext {
  /**
   * @param {StateRepository} stateRepository
   * @param {Buffer} ownerId
   * @param {DataContract} dataContract
   */
  constructor(stateRepository, ownerId, dataContract) {
    /**
     * @type {StateRepository}
     */
    this.stateRepository = stateRepository;
    this.ownerId = EncodedBuffer.from(ownerId, EncodedBuffer.ENCODING.BASE58);
    this.dataContract = dataContract;
  }

  /**
   * @returns {StateRepository}
   */
  getStateRepository() {
    return this.stateRepository;
  }

  /**
   * @returns {EncodedBuffer}
   */
  getOwnerId() {
    return this.ownerId;
  }

  /**
   * @returns {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }
}

module.exports = DataTriggerExecutionContext;
