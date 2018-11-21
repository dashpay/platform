const serializer = require('../util/serializer');

const STPacketHeader = require('./STPacketHeader');

const InvalidSTPacketHeaderStructureError = require('./errors/InvalidSTPacketHeaderStructureError');

class STPacketHeaderFactory {
  constructor(validateSTPacketHeader) {
    this.validateSTPacketHeader = validateSTPacketHeader;
  }

  create(dapContractId, itemsMerkleRoot, itemsHash) {
    return new STPacketHeader(
      dapContractId,
      itemsMerkleRoot,
      itemsHash,
    );
  }

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
