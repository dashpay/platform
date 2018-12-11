const lodashQuery = require('lodash-query');

const AbstractDataProvider = require('./AbstractDataProvider');

class LowDbDataProvider extends AbstractDataProvider {
  /**
   * @param {Object} db LowDB database
   */
  constructor(db) {
    super();

    this.db = db;

    lodashQuery(db._);

    this.db.defaults({
      [LowDbDataProvider.COLLECTIONS.TRANSACTIONS]: [],
      [LowDbDataProvider.COLLECTIONS.DAP_CONTRACTS]: [],
      [LowDbDataProvider.COLLECTIONS.DAP_OBJECTS]: {},
    }).write();
  }

  /**
   * Fetch Dap Contract
   *
   * @param {string} id
   * @return {DapContract|null}
   */
  fetchDapContract(id) {
    return this.getCollection(LowDbDataProvider.COLLECTIONS.DAP_CONTRACTS)
      .find(c => c.getId() === id)
      .value();
  }

  /**
   * Fetch DAP Objects
   *
   * @param {string} dapContractId
   * @param {string} type
   * @param {{ where: Object }} [options]
   * @return {DapObject[]}
   */
  fetchDapObjects(dapContractId, type, options = {}) {
    const collection = this.getCollection(LowDbDataProvider.COLLECTIONS.DAP_OBJECTS);

    if (!collection.has(dapContractId).value()) {
      return [];
    }

    // TODO Implement limit, sort and other fetchDapObjects options
    // TODO Impossible to filter by data properties

    return collection.get(dapContractId)
      .query({ ...(options.where || {}), type })
      .value();
  }

  /**
   * Fetch transaction by ID
   *
   * @param {string} id
   * @return {{ confirmations: number }|null}
   */
  fetchTransaction(id) {
    return this.getCollection(LowDbDataProvider.COLLECTIONS.TRANSACTIONS)
      .find({ id })
      .value();
  }

  /**
   * Set transactions
   *
   * @param {Object[]} transactions
   * @return {LowDbDataProvider}
   */
  setTransactions(transactions) {
    this.db.set(LowDbDataProvider.COLLECTIONS.TRANSACTIONS, [])
      .get(LowDbDataProvider.COLLECTIONS.TRANSACTIONS)
      .push(...transactions)
      .write();

    return this;
  }

  /**
   * Set DAP Contracts
   *
   * @param {DapContract[]} dapContracts
   * @return {LowDbDataProvider}
   */
  setDapContracts(dapContracts) {
    this.db.set(LowDbDataProvider.COLLECTIONS.DAP_CONTRACTS, [])
      .get(LowDbDataProvider.COLLECTIONS.DAP_CONTRACTS)
      .push(...dapContracts)
      .write();

    return this;
  }

  /**
   * Set DAP Objects
   *
   * @param {string} dapContractId
   * @param {DapObject[]} dapObjects
   * @return {LowDbDataProvider}
   */
  setDapObjects(dapContractId, dapObjects) {
    // TODO Dirty hack to set "id" property
    //  We need public properties for Dap Object?
    dapObjects.forEach(o => o.getId());

    this.getCollection(LowDbDataProvider.COLLECTIONS.DAP_OBJECTS)
      .set(dapContractId, [])
      .get(dapContractId)
      .push(...dapObjects)
      .write();

    return this;
  }

  /**
   * Get LowDB collection
   *
   * @param {string} collection
   * @return {Object}
   */
  getCollection(collection) {
    return this.db.get(collection);
  }
}

LowDbDataProvider.COLLECTIONS = {
  TRANSACTIONS: 'transactions',
  DAP_CONTRACTS: 'dapContracts',
  DAP_OBJECTS: 'dapObjects',
};

module.exports = LowDbDataProvider;
