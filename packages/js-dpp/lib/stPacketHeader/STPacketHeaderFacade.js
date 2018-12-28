const validateSTPacketHeaderFactory = require('./validateSTPacketHeaderFactory');

const STPacketHeaderFactory = require('./STPacketHeaderFactory');

class STPacketHeaderFacade {
  /**
   * @param {JsonSchemaValidator} validator
   */
  constructor(validator) {
    this.validateSTPacketHeader = validateSTPacketHeaderFactory(validator);

    this.factory = new STPacketHeaderFactory(this.validateSTPacketHeader);
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
   * @param {Object} rawSTPacketHeader
   * @return {STPacketHeader}
   */
  createFromObject(rawSTPacketHeader) {
    return this.factory.createFromObject(rawSTPacketHeader);
  }

  /**
   * Unserialize ST Packet Header
   *
   * @param {Buffer|string} payload
   * @return {STPacketHeader}
   */
  createFromSerialized(payload) {
    return this.factory.createFromSerialized(payload);
  }

  /**
   *
   * @param {STPacket|Object} stPacketHeader
   * @return {ValidationResult}
   */
  validate(stPacketHeader) {
    return this.validateSTPacketHeader(stPacketHeader);
  }
}

module.exports = STPacketHeaderFacade;
