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
      this.dapContracts[dapContract.getId()] = dapContract;
    });

    this.dapObjects = {};
    dapObjects.forEach((dapObject) => {
      this.dapObjects[`${dapObject.getType()}:${dapObject.getPrimaryKey()}`] = dapObject;
    });
  }

  /**
   * Fetch Dap Contract
   *
   * @param {string} id
   * @return {DapContract|null}
   */
  fetchDapContract(id) {
    return this.dapContracts[id] || null;
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

      if (this.dapObjects[id]) {
        dapObjects.push(this.dapObjects[id]);
      }
    });

    return dapObjects;
  }
}

module.exports = ObjectDataProvider;
