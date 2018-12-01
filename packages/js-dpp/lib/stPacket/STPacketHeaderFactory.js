const serializer = require('../util/serializer');

const STPacketHeader = require('./STPacketHeader');

const InvalidSTPacketHeaderStructureError = require('./errors/InvalidSTPacketHeaderStructureError');

class STPacketHeaderFactory {
  /**
   * @param {validateSTPacketHeader} validateSTPacketHeader
   */
  constructor(validateSTPacketHeader) {
    this.validateSTPacketHeader = validateSTPacketHeader;
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
    return new STPacketHeader(
      dapContractId,
      itemsMerkleRoot,
      itemsHash,
    );
  }

  /**
   * Create ST Packet Header from plain object
   *
   * @param {Object} object
   * @return {STPacketHeader}
   */
  createFromObject(object) {
    const errors = this.validateSTPacketHeader(object);

    if (errors.length) {
      throw new InvalidSTPacketHeaderStructureError(errors, object);
    }

    return this.create(object.contractId, object.itemsMerkleRoot, object.itemsHash);
  }

  /**
   * Unserialize ST Packet Header
   *
   * @param {Buffer|string} payload
   * @return {STPacket}
   */
  createFromSerialized(payload) {
    const object = serializer.decode(payload);
    return this.createFromObject(object);
  }
}

module.exports = STPacketHeaderFactory;
