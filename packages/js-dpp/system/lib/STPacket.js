const DapContract = require('./DapContract');
const DapObject = require('./DapObject');
const InvalidSTPacketStructureError = require('./errors/InvalidSTPacketStructureError');

class STPacket {
  /**
   * @param {string} dapContractId
   */
  constructor(dapContractId) {
    this.setDapContractId(dapContractId);

    this.dapObjects = [];
    this.dapContracts = [];
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

  // TODO How to remove dap contract?

  /**
   * Set Dap Contract
   *
   * @param {DapContract} dapContract
   */
  setDapContract(dapContract) {
    this.dapContracts = [dapContract];
    // TODO: set contract id
    // this.dapContractId = toHash(dapContract);
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
   * Return ST Packet as plain object
   *
   * @return {{dapContractId: string, [dapContracts]: Object[], [dapObjects]: Object[]}}
   */
  toJSON() {
    // TODO: Validate before to JSON ?
    return {
      dapContractId: this.getDapContractId(),
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
