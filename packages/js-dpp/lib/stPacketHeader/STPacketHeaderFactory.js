const { decode } = require('../util/serializer');

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
   * @param {string} contractId
   * @param {string} itemsMerkleRoot
   * @param {string} itemsHash
   * @return {STPacketHeader}
   */
  create(contractId, itemsMerkleRoot, itemsHash) {
    return new STPacketHeader(
      contractId,
      itemsMerkleRoot,
      itemsHash,
    );
  }

  /**
   * Create ST Packet Header from plain object
   *
   * @param {Object} rawSTPacketHeader
   * @return {STPacketHeader}
   */
  createFromObject(rawSTPacketHeader) {
    const result = this.validateSTPacketHeader(rawSTPacketHeader);

    if (!result.isValid()) {
      throw new InvalidSTPacketHeaderError(result.getErrors(), rawSTPacketHeader);
    }

    return this.create(
      rawSTPacketHeader.contractId,
      rawSTPacketHeader.itemsMerkleRoot,
      rawSTPacketHeader.itemsHash,
    );
  }

  /**
   * Unserialize ST Packet Header
   *
   * @param {Buffer|string} payload
   * @return {STPacketHeader}
   */
  createFromSerialized(payload) {
    const rawSTPacketHeader = decode(payload);

    return this.createFromObject(rawSTPacketHeader);
  }
}

module.exports = STPacketHeaderFactory;
