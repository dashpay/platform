const AbstractDataProvider = require('./AbstractDataProvider');

class ObjectDataProvider extends AbstractDataProvider {
  /**
   * @param {DapContract[]}dapContracts
   * @param {DapObject[]} dapObjects
   */
  constructor({ dapContracts, dapObjects }) {
    super();

    this.dapContracts = {};
    dapContracts.forEach((dapContract) => {
      this.contracts[dapContract.getId()] = dapContract;
    });

    this.dapObjects = {};
    dapObjects.forEach((dapObject) => {
      this.objects[`${dapObject.getType()}:${dapObject.getPrimaryKey()}`] = dapObject;
    });
  }

  /**
   * Fetch Dap Contract
   *
   * @param {string} id
   * @return {DapContract|null}
   */
  fetchDapContract(id) {
    return this.contracts[id] || null;
  }

  /**
   * Fetch Dap Objects
   *
   * @param {[{type: string, primaryKey: string}]} primaryKeysAndTypes
   * @return {DapObject[]}
   */
  fetchDapObjects(primaryKeysAndTypes) {
    const dapObjects = [];

    primaryKeysAndTypes.forEach((primaryKeyAndType) => {
      const { type, primaryKey } = primaryKeyAndType;

      const id = `${type}:${primaryKey}`;

      if (this.objects[id]) {
        dapObjects.push(this.objects[id]);
      }
    });

    return dapObjects;
  }
}

module.exports = ObjectDataProvider;
