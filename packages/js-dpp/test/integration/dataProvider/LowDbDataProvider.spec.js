const LowDb = require('lowdb');
const Memory = require('lowdb/adapters/Memory');

const LowDbDataProvider = require('../../../lib/dataProvider/LowDbDataProvider');

const DapObject = require('../../../lib/dapObject/DapObject');
const DapContract = require('../../../lib/dapContract/DapContract');

describe('LowDbDataProvider', () => {
  let db;
  let dataProvider;

  beforeEach(() => {
    const adapter = new Memory();
    db = new LowDb(adapter);

    dataProvider = new LowDbDataProvider(db);
  });

  it('should create default collections', () => {
    const transactions = db.get(LowDbDataProvider.COLLECTIONS.TRANSACTIONS).value();
    expect(transactions).to.be.an('array');

    const dapContracts = db.get(LowDbDataProvider.COLLECTIONS.DAP_CONTRACTS).value();
    expect(dapContracts).to.be.an('array');

    const dapObjects = db.get(LowDbDataProvider.COLLECTIONS.DAP_OBJECTS).value();
    expect(dapObjects).to.be.deep.equal({});
  });

  it('should set and get transactions', () => {
    const transactions = [
      { id: 'e56d336922eaab3be8c1244dbaa713e134a8eba50ddbd4f50fd2fe18d72595cd', confirmations: 3 },
      { id: '0cc6331b7505b1a43798cf3b69af7dd02dfef9e1f9922d4152023c486c377a46', confirmations: 2 },
      { id: '16ecab1875791e2b6ed0c9a6dae5a12a79d92120e1c3afbd3a9c8535ce44666d', confirmations: 1 },
    ];

    const result = dataProvider.setTransactions(transactions);
    expect(result).to.be.instanceOf(LowDbDataProvider);

    transactions.forEach((transaction) => {
      const actualTransaction = dataProvider.fetchTransaction(transaction.id);
      expect(actualTransaction).to.be.deep.equal(transaction);
    });
  });

  it('should set and get DAP Objects', () => {
    const type = 'test';
    const dapContract = new DapContract('test', {
      [type]: { },
    });
    const userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

    const dapObjects = [
      new DapObject(dapContract, userId, type, { number: 1 }),
      new DapObject(dapContract, userId, type, { number: 2 }),
      new DapObject(dapContract, userId, type, { number: 3 }),
    ];

    const result = dataProvider.setDapObjects(dapContract.getId(), dapObjects);
    expect(result).to.be.instanceOf(LowDbDataProvider);

    const allDapObjects = dataProvider.fetchDapObjects(
      dapContract.getId(),
      type,
    );
    expect(allDapObjects).to.be.deep.equal(dapObjects);

    dapObjects.forEach((dapObject) => {
      const actualDapObjects = dataProvider.fetchDapObjects(
        dapContract.getId(),
        type,
        { where: { id: { $in: [dapObject.getId()] } } },
      );

      expect(actualDapObjects).to.be.a('array').and.lengthOf(1);
      expect(actualDapObjects[0]).to.be.deep.equal(dapObject);
    });
  });

  it('should set and get DAP Contracts', () => {
    const dapContracts = [
      new DapContract('test1', {}),
      new DapContract('test2', {}),
      new DapContract('test3', {}),
    ];

    const result = dataProvider.setDapContracts(dapContracts);
    expect(result).to.be.instanceOf(LowDbDataProvider);

    dapContracts.forEach((dapContract) => {
      const actualDapContract = dataProvider.fetchDapContract(dapContract.getId());

      expect(actualDapContract).to.be.deep.equal(dapContract);
    });
  });
});
