const AbstractDataProvider = require('./AbstractDataProvider');

class ObjectDataProvider extends AbstractDataProvider {
  /**
   * Set Dap Contracts
   *
   * @param {DapContract[]} dapContracts
   * @return {ObjectDataProvider}
   */
  setDapContracts(dapContracts) {
    this.dapContracts = {};
    dapContracts.forEach((dapContract) => {
      this.dapContracts[dapContract.getId()] = dapContract;
    });

    return this;
  }

  /**
   * Set Dap Objects
   *
   * @param {DapObject[]} dapObjects
   * @return {ObjectDataProvider}
   */
  setDapObjects(dapObjects) {
    this.dapObjects = {};
    dapObjects.forEach((dapObject) => {
      this.dapObjects[`${dapObject.getType()}:${dapObject.getPrimaryKey()}`] = dapObject;
    });

    return this;
  }

  /**
   * Set transactions
   *
   * @param {{ confirmations: number }} transactions
   * @return {ObjectDataProvider}
   */
  setTransactions(transactions) {
    this.transactions = {};
    transactions.forEach((transaction) => {
      this.transactions[transaction.id] = transaction;
    });

    return this;
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

  /**
   * Get transaction by ID
   *
   * @param {string} id
   * @return {{ confirmations: number }}
   */
  getTransaction(id) {
    return this.transactions[id] || null;
  }
}

module.exports = ObjectDataProvider;
