const serializer = require('../util/serializer');

const STPacketHeader = require('./STPacketHeader');

const InvalidSTPacketHeaderError = require('../stPacket/errors/InvalidSTPacketHeaderError');

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
    const result = this.validateSTPacketHeader(object);

    if (result.isValid()) {
      throw new InvalidSTPacketHeaderError(result.getErrors(), object);
    }

    return this.create(object.contractId, object.itemsMerkleRoot, object.itemsHash);
  }

  /**
   * Unserialize ST Packet Header
   *
   * @param {Buffer|string} payload
   * @return {STPacketHeader}
   */
  createFromSerialized(payload) {
    const object = serializer.decode(payload);

    return this.createFromObject(object);
  }
}

module.exports = STPacketHeaderFactory;
