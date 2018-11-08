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
   * @param {string} contractId
   */
  setDapContractId(contractId) {
    this.contractId = contractId;
  }

  /**
   * Get Dap Contract ID
   *
   * @return {string}
   */
  getDapContractId() {
    return this.contractId;
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
   * @return {{contractId: string, itemsMerkleRoot: string, itemsHash: string}}
   */
  toJSON() {
    // TODO: Validate before to JSON ?
    return {
      contractId: this.getDapContractId(),
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
   * Returns hex string with ST packet header hash
   *
   * @return {string}
   */
  hash() {
    return STPacketHeader.hashingFunction(this.serialize());
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

    return new STPacketHeader(object.contractId, object.itemsMerkleRoot, object.itemsHash);
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

  /**
   * Set hashing function
   *
   * @param {function(Buffer):string}  hashingFunction
   */
  static setHashingFunction(hashingFunction) {
    STPacketHeader.hashingFunction = hashingFunction;
  }
}

module.exports = STPacketHeader;
