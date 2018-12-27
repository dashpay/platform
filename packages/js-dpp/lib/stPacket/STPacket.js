const hash = require('../util/hash');
const serializer = require('../util/serializer');

const DapContract = require('../dapContract/DapContract');

const ContractAndObjectsNotAllowedSamePacketError = require('./errors/ContractAndObjectsNotAllowedSamePacketError');

class STPacket {
  /**
   * @param {string} contractId
   * @param {DapContract|DapObject[]} [items] DAP Contract or DAP Objects
   */
  constructor(contractId, items = undefined) {
    this.setDapContractId(contractId);

    this.itemsMerkleRoot = null;
    this.itemsHash = null;

    this.objects = [];
    this.contracts = [];

    if (items instanceof DapContract) {
      this.setDapContract(items);
    }

    if (Array.isArray(items)) {
      this.setDapObjects(items);
    }
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
   * Set Dap Contract
   *
   * @param {DapContract} dapContract
   */
  setDapContract(dapContract) {
    if (this.objects.length > 0) {
      throw new ContractAndObjectsNotAllowedSamePacketError(this);
    }

    this.contracts = !dapContract ? [] : [dapContract];

    return this;
  }

  /**
   * Get Dap Contract
   *
   * @return {DapContract|null}
   */
  getDapContract() {
    if (this.contracts.length) {
      return this.contracts[0];
    }

    return null;
  }

  /**
   * Set Dap Objects
   *
   * @param {DapObject[]} dapObjects
   */
  setDapObjects(dapObjects) {
    if (this.contracts.length) {
      throw new ContractAndObjectsNotAllowedSamePacketError(this);
    }

    this.objects = dapObjects;

    return this;
  }

  /**
   * Get Dap Objects
   *
   * @return {DapObject[]}
   */
  getDapObjects() {
    return this.objects;
  }

  /**
   * Add Dap Object
   *
   * @param {DapObject...} dapObjects
   */
  addDapObject(...dapObjects) {
    this.objects.push(...dapObjects);

    return this;
  }

  /**
   * Return ST Packet as plain object
   *
   * @return {{contractId: string,
   *           itemsMerkleRoot: string,
   *           itemsHash: string,
   *           contracts: Object[],
   *           objects: Object[]}}
   */
  toJSON() {
    return {
      contractId: this.getDapContractId(),
      itemsMerkleRoot: this.getItemsMerkleRoot(),
      itemsHash: this.getItemsHash(),
      contracts: this.contracts.map(dapContract => dapContract.toJSON()),
      objects: this.objects.map(dapObject => dapObject.toJSON()),
    };
  }

  /**
   * Return serialized ST Packet
   *
   * @return {Buffer}
   */
  serialize() {
    return serializer.encode(this.toJSON());
  }

  /**
   * Returns hex string with ST packet hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize());
  }
}

module.exports = STPacket;
