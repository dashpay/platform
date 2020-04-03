const level = require('level-rocksdb');
const cbor = require('cbor');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

const LevelDBTransaction = require('../../../lib/levelDb/LevelDBTransaction');

const DataContractLevelDBRepository = require('../../../lib/dataContract/DataContractLevelDBRepository');

describe('DataContractLevelDBRepository', () => {
  let db;
  let repository;
  let dataContract;
  let key;
  let dppMock;

  beforeEach(function beforeEach() {
    db = level('./db/data-contract-test', { valueEncoding: 'binary' });

    dataContract = getDataContractFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock
      .dataContract
      .createFromSerialized
      .resolves(dataContract);

    repository = new DataContractLevelDBRepository(db, dppMock);

    key = `${DataContractLevelDBRepository.KEY_PREFIX}:${dataContract.getId()}`;
  });

  afterEach(async () => {
    await db.clear();
    await db.close();
  });

  describe('#store', () => {
    it('should store data contract', async () => {
      const repositoryInstance = await repository.store(dataContract);

      expect(repositoryInstance).to.equal(repository);

      const storedDataContractBuffer = await db.get(key);

      expect(storedDataContractBuffer).to.be.instanceOf(Buffer);

      const storedDataContract = cbor.decode(storedDataContractBuffer);

      expect(storedDataContract).to.deep.equal(dataContract.toJSON());
    });

    it('should store data contract in transaction', async () => {
      const transaction = repository.createTransaction();

      expect(transaction).to.be.instanceOf(LevelDBTransaction);

      transaction.start();

      // store data in transaction
      await repository.store(dataContract, transaction);

      // check we don't have data in db before commit
      try {
        await db.get(key);

        expect.fail('Should fail with NotFoundError error');
      } catch (e) {
        expect(e.type).to.equal('NotFoundError');
      }

      // check we can't fetch data without transaction
      const notFoundDataContract = await repository.fetch(key);

      expect(notFoundDataContract).to.be.null();

      // check we can fetch data inside transaction
      const dataContractFromTransaction = await repository.fetch(dataContract.getId(), transaction);

      expect(dataContractFromTransaction).to.be.instanceOf(DataContract);
      expect(dataContractFromTransaction.toJSON()).to.deep.equal(dataContract.toJSON());

      await transaction.commit();

      // check we have data in db after commit
      const storedDataContractBuffer = await db.get(key);

      expect(storedDataContractBuffer).to.be.instanceOf(Buffer);

      const storedDataContract = cbor.decode(storedDataContractBuffer);

      expect(storedDataContract).to.deep.equal(dataContract.toJSON());
    });
  });

  describe('#fetch', () => {
    it('should return null if data contract was not found', async () => {
      await repository.store(dataContract);

      const storedDataContract = await repository.fetch('nonExistingId');

      expect(storedDataContract).to.be.null();
    });

    it('should return stored data contract', async () => {
      await db.put(key, dataContract.serialize());

      const storedDataContract = await repository.fetch(dataContract.getId());

      expect(storedDataContract.toJSON()).to.deep.equal(dataContract.toJSON());
    });

    it('should return stored data contract with transaction', async () => {
      await repository.store(dataContract);

      const transaction = repository.createTransaction();

      transaction.start();

      const storedDataContract = await repository.fetch(dataContract.getId(), transaction);

      expect(storedDataContract.toJSON()).to.deep.equal(dataContract.toJSON());
    });

    it('should return null if data contract not found', async () => {
      const storedDataContract = await repository.fetch(null);

      expect(storedDataContract).to.equal(null);
    });
  });
});
