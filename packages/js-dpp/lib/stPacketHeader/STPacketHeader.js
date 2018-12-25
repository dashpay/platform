const hash = require('../util/hash');
const { encode } = require('../util/serializer');

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

    return this;
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
   * @return {{contractId: string, itemsMerkleRoot: string, itemsHash: string}}
   */
  toJSON() {
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
    return encode(this.toJSON());
  }

  /**
   * Returns hex string with ST packet header hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize());
  }
}

module.exports = STPacketHeader;
