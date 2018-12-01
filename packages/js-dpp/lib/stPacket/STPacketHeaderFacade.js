const validateSTPacketHeaderFactory = require('./validation/validateSTPacketHeaderFactory');

const STPacketHeaderFactory = require('./STPacketHeaderFactory');

class STPacketHeaderFacade {
  /**
   * @param {SchemaValidator} validator
   */
  constructor(validator) {
    this.validateSTPacketHeader = validateSTPacketHeaderFactory(validator);

    this.factory = new STPacketHeaderFactory(this.validateSTPacketHeader);
  }

  updateDependencies() {

  }

  /**
   * Create ST Packet Header
   *
   * @param {string} dapContractId
   * @param {string} itemsMerkleRoot
   * @param {string} itemsHash
   * @return {STPacketHeader}
   */
  create(dapContractId, itemsMerkleRoot, itemsHash) {
    return this.factory.create(dapContractId, itemsMerkleRoot, itemsHash);
  }

  /**
   * Create ST Packet Header from plain object
   *
   * @param {Object} object
   * @return {STPacketHeader}
   */
  createFromObject(object) {
    return this.factory.createFromObject(object);
  }

  /**
   * Unserialize ST Packet Header
   *
   * @param {Buffer|string} payload
   * @return {STPacket}
   */
  createFromSerialized(payload) {
    return this.factory.createFromSerialized(payload);
  }

  /**
   *
   * @param {STPacket|Object} stPacketHeader
   * @return {Object[]|*}
   */
  validate(stPacketHeader) {
    return this.validateSTPacketHeader(stPacketHeader);
  }
}

module.exports = STPacketHeaderFacade;
