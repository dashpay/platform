/**
 * @class STPacket
 * @property {string} contractId
 * @property Array<Object> dapContracts
 * @property Array<Object> dapObjects
 */
class STPacket {
  constructor() {
    this.dapObjects = [];
    this.dapContracts = [];
  }

  /**
   * @param {DapContract} dapContract
   */
  setDapContract(dapContract) {
    this.dapContracts = [dapContract];
    // TODO: set contract id
    // this.contractId = toHash(dapContract);
  }

  /**
   * @template TDapObject {Object}
   * @param {Array<TDapObject>} dapObjects
   */
  setDapObjects(dapObjects) {
    this.dapObjects = dapObjects;
  }

  /**
   * @template TDapObject {Object}
   * @param {Array<TDapObject>} dapObjects
   */
  addDapObject(dapObjects) {
    this.dapObjects.push(...dapObjects);
  }

  /**
   * @param {string} contractId
   */
  setContractId(contractId) {
    this.contractId = contractId;
  }

  /**
   * @return {{contractId: string, dapContracts: Array<Object>, dapObjects: Array<Object>}}
   */
  toJson() {
    return {
      contractId: this.contractId,
      dapContracts: this.dapContracts,
      dapObjects: this.dapObjects,
    };
  }
}

module.exports = STPacket;
