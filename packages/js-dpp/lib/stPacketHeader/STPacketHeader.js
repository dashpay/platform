const hash = require('../util/hash');
const { encode } = require('../util/serializer');

class STPacketHeader {
  /**
   * @param {string} contractId
   * @param {string} itemsMerkleRoot
   * @param {string} itemsHash
   */
  constructor(contractId, itemsMerkleRoot, itemsHash) {
    this.setContractId(contractId);
    this.setItemsMerkleRoot(itemsMerkleRoot);
    this.setItemsHash(itemsHash);
  }

  /**
   * Set Contract ID
   *
   * @param {string} contractId
   */
  setContractId(contractId) {
    this.contractId = contractId;

    return this;
  }

  /**
   * Get Contract ID
   *
   * @return {string}
   */
  getContractId() {
    return this.contractId;
  }

  /**
   * Set items merkle root
   *
   * @param {string} itemsMerkleRoot
   */
  setItemsMerkleRoot(itemsMerkleRoot) {
    this.itemsMerkleRoot = itemsMerkleRoot;

    return this;
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

    return this;
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
   * @return {RawSTPacketHeader}
   */
  toJSON() {
    return {
      contractId: this.getContractId(),
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
    return encode(this.toJSON());
  }

  /**
   * Returns hex string with ST packet header hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize()).toString('hex');
  }
}

module.exports = STPacketHeader;
