const DapContract = require('./DapContract');
const DapObject = require('./DapObject');

/**
 * @class STPacket
 * @property {string} dapContractId
 * @property Array<Object> dapContracts
 * @property Array<Object> dapObjects
 */
class STPacket {
  constructor(contractId) {
    this.setDapContractId(contractId);

    this.dapObjects = [];
    this.dapContracts = [];
  }

  /**
   * @param {string} dapContractId
   */
  setDapContractId(dapContractId) {
    this.dapContractId = dapContractId;
  }

  /**
   *
   * @return {string}
   */
  getDapContractId() {
    return this.dapContractId;
  }

  /**
   * @param {DapContract} dapContract
   */
  setDapContract(dapContract) {
    this.dapContracts = [dapContract];
    // TODO: set contract id
    // this.dapContractId = toHash(dapContract);
  }

  /**
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
   * @param {DapObject[]} dapObjects
   */
  setDapObjects(dapObjects) {
    this.dapObjects = dapObjects;
  }

  /**
   *
   * @return {DapObject[]}
   */
  getDapObjects() {
    return this.dapObjects;
  }

  /**
   * @template TDapObject {Object}
   * @param {Array<TDapObject>} dapObjects
   */
  addDapObject(dapObjects) {
    this.dapObjects.push(...dapObjects);
  }

  /**
   * @return {{dapContractId: string, dapContracts: Object[], dapObjects: Object[]}}
   */
  toJSON() {
    return {
      dapContractId: this.dapContractId,
      dapContracts: this.dapContracts.map(dapContract => dapContract.toJSON()),
      dapObjects: this.dapObjects.map(dapObject => dapObject.toJSON()),
    };
  }

  /**
   *
   * @param {Object} object
   * @return {STPacket}
   */
  static fromObject(object) {
    const errors = STPacket.structureValidator(object);

    if (errors.length) {
      throw new Error(errors);
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

  static setSerializer(serializer) {
    STPacket.serializer = serializer;
  }

  static setStructureValidator(validator) {
    STPacket.structureValidator = validator;
  }
}

module.exports = STPacket;
