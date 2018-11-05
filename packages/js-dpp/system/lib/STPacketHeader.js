const InvalidSTPacketHeaderStructureError = require('./errors/InvalidSTPacketHeaderStructureError');

class STPacketHeader {
  /**
   * @param {string} dapContractId
   * @param {string} itemsMerkleRoot
   * @param {string} itemsHash
   */
  constructor(dapContractId, itemsMerkleRoot, itemsHash) {
    this.setDapContractId(dapContractId);
    this.setItemsMerkleRoot(itemsMerkleRoot);
    this.setItemsHash(itemsHash);
  }

  /**
   * Set Dap Contract ID
   *
   * @param {string} dapContractId
   */
  setDapContractId(dapContractId) {
    this.dapContractId = dapContractId;
  }

  /**
   * Get Dap Contract ID
   *
   * @return {string}
   */
  getDapContractId() {
    return this.dapContractId;
  }

  /**
   * Set items merkle root
   *
   * @param {string} itemsMerkleRoot
   */
  setItemsMerkleRoot(itemsMerkleRoot) {
    this.itemsMerkleRoot = itemsMerkleRoot;
  }

  /**
   * Get items merkle root
   *
   * @return {string}
   */
  getItemsMerkleRoot() {
    return this.itemsMerkleRoot;
  }

  /**
   * Set items hash
   *
   * @param {string} itemsHash
   */
  setItemsHash(itemsHash) {
    this.itemsHash = itemsHash;
  }

  /**
   * Get items hash
   *
   * @return {string}
   */
  getItemsHash() {
    return this.itemsHash;
  }

  /**
   * Return ST Packet Header as plain object
   *
   * @return {{dapContractId: string, itemsMerkleRoot: string, itemsHash: string}}
   */
  toJSON() {
    // TODO: Validate before to JSON ?
    return {
      dapContractId: this.getDapContractId(),
      itemsMerkleRoot: this.getItemsMerkleRoot(),
      itemsHash: this.getItemsHash(),
    };
  }

  /**
   * Return serialized ST Packet
   *
   * @return {Buffer}
   */
  serialize() {
    // TODO: Validate before serialization ?
    return STPacketHeader.serializer.encode(this.toJSON());
  }

  /**
   *
   * @param {Object} object
   * @return {STPacketHeader}
   */
  static fromObject(object) {
    const errors = STPacketHeader.structureValidator(object);

    if (errors.length) {
      throw new InvalidSTPacketHeaderStructureError(errors, object);
    }

    return new STPacketHeader(object.dapContractId, object.itemsMerkleRoot, object.itemsHash);
  }

  /**
   *
   * @param {Buffer|string} payload
   * @return {STPacketHeader}
   */
  static fromSerialized(payload) {
    const object = STPacketHeader.serializer.decode(payload);
    return STPacketHeader.fromObject(object);
  }

  /**
   * Set serializer
   *
   * @param {serializer} serializer
   */
  static setSerializer(serializer) {
    STPacketHeader.serializer = serializer;
  }

  /**
   * Set structure validator
   *
   * @param {Function} validator
   */
  static setStructureValidator(validator) {
    STPacketHeader.structureValidator = validator;
  }
}

module.exports = STPacketHeader;
