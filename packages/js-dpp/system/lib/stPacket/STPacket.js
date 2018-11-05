const DapContract = require('../dapContract/DapContract');
const DapObject = require('../dapObject/DapObject');
const STPacketHeader = require('./STPacketHeader');

const InvalidSTPacketStructureError = require('./errors/InvalidSTPacketStructureError');
const EitherDapContractOrDapObjectsAllowedError = require('./errors/EitherDapContractOrDapObjectsAllowedError');

class STPacket {
  /**
   * @param {string} dapContractId
   */
  constructor(dapContractId) {
    this.setDapContractId(dapContractId);

    this.itemsMerkleRoot = null;
    this.itemsHash = null;

    this.dapObjects = [];
    this.dapContracts = [];
  }

  // TODO Reuse code from STPacketHeader ?

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
   * Set Dap Contract
   *
   * @param {DapContract} dapContract
   */
  setDapContract(dapContract) {
    if (this.dapObjects.length) {
      throw new EitherDapContractOrDapObjectsAllowedError(this);
    }

    this.dapContracts = !dapContract ? [] : [dapContract];

    // TODO: set contract id
    // this.dapContractId = toHash(dapContract);

    return this;
  }

  /**
   * Get Dap Contract
   *
   * @return {DapContract|null}
   */
  getDapContract() {
    if (this.dapContracts.length) {
      return this.dapContracts[0];
    }

    return null;
  }

  /**
   * Set Dap Objects
   *
   * @param {DapObject[]} dapObjects
   */
  setDapObjects(dapObjects) {
    if (this.dapContracts.length) {
      throw new EitherDapContractOrDapObjectsAllowedError(this);
    }

    this.dapObjects = dapObjects;
  }

  /**
   * Get Dap Objects
   *
   * @return {DapObject[]}
   */
  getDapObjects() {
    return this.dapObjects;
  }

  /**
   * Add Dap Object
   *
   * @param {DapObject...} dapObjects
   */
  addDapObject(...dapObjects) {
    this.dapObjects.push(...dapObjects);
  }

  /**
   * Create STPacketHeader with STPacket data
   *
   * @return STPacketHeader
   */
  extractHeader() {
    return STPacketHeader(
      this.getDapContractId(),
      this.getItemsMerkleRoot(),
      this.getItemsHash(),
    );
  }

  /**
   * Return ST Packet as plain object
   *
   * @return {{dapContractId: string, dapContracts: Object[], dapObjects: Object[]}}
   */
  toJSON() {
    // TODO: Validate before to JSON ?
    return {
      dapContractId: this.getDapContractId(),
      itemsMerkleRoot: this.getItemsMerkleRoot(),
      itemsHash: this.getItemsHash(),
      dapContracts: this.dapContracts.map(dapContract => dapContract.toJSON()),
      dapObjects: this.dapObjects.map(dapObject => dapObject.toJSON()),
    };
  }

  /**
   * Return serialized ST Packet
   *
   * @return {Buffer}
   */
  serialize() {
    // TODO: Validate before serialization ?
    return STPacket.serializer.encode(this.toJSON());
  }

  /**
   *
   * @param {Object} object
   * @return {STPacket}
   */
  static fromObject(object) {
    const errors = STPacket.structureValidator(object);

    if (errors.length) {
      throw new InvalidSTPacketStructureError(errors, object);
    }

    const stPacket = new STPacket(object.dapContractId);

    stPacket.setItemsMerkleRoot(object.itemsMerkleRoot);
    stPacket.setItemsHash(object.itemsHash);

    if (object.dapContracts.length) {
      const dapContract = DapContract.fromObject(object.dapContracts[0]);

      stPacket.setDapContract(dapContract);
    }

    if (object.dapObjects.length) {
      const dapObjects = object.dapObjects.map(dapObject => DapObject.fromObject(dapObject));
      stPacket.setDapObjects(dapObjects);
    }

    return stPacket;
  }

  /**
   *
   * @param {Buffer|string} payload
   * @return {STPacket}
   */
  static fromSerialized(payload) {
    const object = STPacket.serializer.decode(payload);
    return STPacket.fromObject(object);
  }

  /**
   * Set serializer
   *
   * @param {serializer} serializer
   */
  static setSerializer(serializer) {
    STPacket.serializer = serializer;
  }

  /**
   * Set structure validator
   *
   * @param {Function} validator
   */
  static setStructureValidator(validator) {
    STPacket.structureValidator = validator;
  }
}

module.exports = STPacket;
