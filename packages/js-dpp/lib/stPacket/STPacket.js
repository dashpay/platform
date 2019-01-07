const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const calculateItemsMerkleRoot = require('./calculateItemsMerkleRoot');
const calculateItemsHash = require('./calculateItemsHash');

const DPContract = require('../contract/DPContract');

const ContractAndObjectsNotAllowedSamePacketError = require('./errors/ContractAndObjectsNotAllowedSamePacketError');

class STPacket {
  /**
   * @param {string} contractId
   * @param {DPContract|DPObject[]} [items] DP Contract or DP Objects
   */
  constructor(contractId, items = undefined) {
    this.setDPContractId(contractId);

    this.objects = [];
    this.contracts = [];

    if (items instanceof DPContract) {
      this.setDPContract(items);
    }

    if (Array.isArray(items)) {
      this.setDPObjects(items);
    }
  }

  /**
   * Set DP Contract ID
   *
   * @param {string} contractId
   */
  setDPContractId(contractId) {
    this.contractId = contractId;

    return this;
  }

  /**
   * Get DP Contract ID
   *
   * @return {string}
   */
  getDPContractId() {
    return this.contractId;
  }


  /**
   * Get items merkle root
   *
   * @return {string|null}
   */
  getItemsMerkleRoot() {
    return calculateItemsMerkleRoot({
      contracts: this.contracts,
      objects: this.objects,
    });
  }

  /**
   * Get items hash
   *
   * @return {string}
   */
  getItemsHash() {
    return calculateItemsHash({
      contracts: this.contracts,
      objects: this.objects,
    });
  }

  /**
   * Set DP Contract
   *
   * @param {DPContract} dpContract
   */
  setDPContract(dpContract) {
    if (this.objects.length > 0) {
      throw new ContractAndObjectsNotAllowedSamePacketError(this);
    }

    this.contracts = !dpContract ? [] : [dpContract];

    return this;
  }

  /**
   * Get DP Contract
   *
   * @return {DPContract|null}
   */
  getDPContract() {
    if (this.contracts.length) {
      return this.contracts[0];
    }

    return null;
  }

  /**
   * Set DPObjects
   *
   * @param {DPObject[]} dpObjects
   */
  setDPObjects(dpObjects) {
    if (this.contracts.length) {
      throw new ContractAndObjectsNotAllowedSamePacketError(this);
    }

    this.objects = dpObjects;

    return this;
  }

  /**
   * Get DPObjects
   *
   * @return {DPObject[]}
   */
  getDPObjects() {
    return this.objects;
  }

  /**
   * Add DP Object
   *
   * @param {DPObject...} dpObjects
   */
  addDPObject(...dpObjects) {
    this.objects.push(...dpObjects);

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
      contractId: this.getDPContractId(),
      itemsMerkleRoot: this.getItemsMerkleRoot(),
      itemsHash: this.getItemsHash(),
      contracts: this.contracts.map(dpContract => dpContract.toJSON()),
      objects: this.objects.map(dpObject => dpObject.toJSON()),
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
   * Returns hex string with ST packet hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize());
  }
}

module.exports = STPacket;
