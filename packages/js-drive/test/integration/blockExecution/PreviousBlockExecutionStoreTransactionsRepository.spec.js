const cbor = require('cbor');

const { startMongoDb } = require('@dashevo/dp-services-ctl');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

describe('PreviousBlockExecutionStoreTransactionsRepository', function main() {
  this.timeout(25000);

  let container;
  let mongoDB;
  let repository;
  let fileDb;
  let blockExecutionStoreTransactions;
  let updateKey;
  let updateValue;
  let deleteKey;
  let deleteValue;

  before(async () => {
    mongoDB = await startMongoDb();
  });

  after(async () => {
    await mongoDB.remove();
  });

  beforeEach(async () => {
    container = await createTestDIContainer(mongoDB);

    fileDb = container.resolve('previousBlockExecutionTransactionDB');
    repository = container.resolve('previousBlockExecutionStoreTransactionsRepository');
    blockExecutionStoreTransactions = container.resolve('blockExecutionStoreTransactions');

    await blockExecutionStoreTransactions.start();

    const dataContract = getDataContractFixture();
    const [document] = getDocumentsFixture(dataContract);

    blockExecutionStoreTransactions.getTransaction('dataContracts').db.put(
      dataContract.getId(),
      dataContract.toBuffer(),
    );

    updateKey = Buffer.from([1]);
    updateValue = document.toBuffer();
    deleteKey = Buffer.from([3]);
    deleteValue = document.toBuffer();

    Object.values(blockExecutionStoreTransactions.transactions).forEach((tx) => {
      const db = tx.db ? tx.db : tx.storeTransaction.db;

      db.put(deleteKey, deleteValue);
      db.persist();

      db.put(updateKey, updateValue);
      db.delete(deleteKey);
    });
  });

  afterEach(async () => {
    await container.dispose();
  });

  it('should store all transactions', async () => {
    await repository.store(blockExecutionStoreTransactions);

    const transactionsBuffer = await fileDb.get();

    const transactions = cbor.decode(transactionsBuffer);

    expect(transactions).to.deep.equal(blockExecutionStoreTransactions.toObject());
  });

  it('should fetch all transactions', async () => {
    fileDb.set(
      cbor.encode(blockExecutionStoreTransactions.toObject()),
    );

    const fetched = await repository.fetch();

    expect(fetched.toObject()).to.deep.equals(blockExecutionStoreTransactions.toObject());
  });
});
