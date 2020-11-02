const rimraf = require('rimraf');
const merk = require('merk');
const cbor = require('cbor');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

const MerkDbTransaction = require('../../../lib/merkDb/MerkDbTransaction');

const DataContractMerkDBRepository = require('../../../lib/dataContract/DataContractMerkDBRepository');

describe('DataContractMerkDBRepository', () => {
  let dbPath;
  let db;
  let repository;
  let dataContract;
  let dppMock;

  beforeEach(function beforeEach() {
    dbPath = './db/identity-test';
    db = merk(`${dbPath}/${Math.random()}`);

    dataContract = getDataContractFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock
      .dataContract
      .createFromBuffer
      .resolves(dataContract);

    repository = new DataContractMerkDBRepository(db, dppMock);
  });

  after(async () => {
    rimraf.sync(dbPath);
  });

  describe('#store', () => {
    it('should store data contract', async () => {
      const repositoryInstance = await repository.store(dataContract);

      expect(repositoryInstance).to.equal(repository);

      const storedDataContractBuffer = db.getSync(dataContract.getId());

      expect(storedDataContractBuffer).to.be.instanceOf(Buffer);

      const storedDataContract = cbor.decode(storedDataContractBuffer);

      expect(storedDataContract).to.deep.equal(dataContract.toObject());
    });

    it('should store data contract in transaction', async () => {
      const transaction = repository.createTransaction();

      expect(transaction).to.be.instanceOf(MerkDbTransaction);

      transaction.start();

      // store data in transaction
      await repository.store(dataContract, transaction);

      // check we don't have data in db before commit
      try {
        db.getSync(dataContract.getId());

        expect.fail('Should fail with NotFoundError error');
      } catch (e) {
        expect(e.message.startsWith('key not found')).to.be.true();
      }

      // check we can't fetch data without transaction
      const notFoundDataContract = await repository.fetch(dataContract.getId());

      expect(notFoundDataContract).to.be.null();

      // check we can fetch data inside transaction
      const dataContractFromTransaction = await repository.fetch(dataContract.getId(), transaction);

      expect(dataContractFromTransaction).to.be.instanceOf(DataContract);
      expect(dataContractFromTransaction.toObject()).to.deep.equal(dataContract.toObject());

      await transaction.commit();

      // check we have data in db after commit
      const storedDataContractBuffer = db.getSync(dataContract.getId());

      expect(storedDataContractBuffer).to.be.instanceOf(Buffer);

      const storedDataContract = cbor.decode(storedDataContractBuffer);

      expect(storedDataContract).to.deep.equal(dataContract.toObject());
    });
  });

  describe('#fetch', () => {
    it('should return null if data contract was not found', async () => {
      await repository.store(dataContract);

      const storedDataContract = await repository.fetch(Buffer.alloc(32));

      expect(storedDataContract).to.be.null();
    });

    it('should return stored data contract', async () => {
      db.batch().put(dataContract.getId(), dataContract.toBuffer()).commitSync();

      const storedDataContract = await repository.fetch(dataContract.getId());

      expect(storedDataContract.toObject()).to.deep.equal(dataContract.toObject());
    });

    it('should return stored data contract with transaction', async () => {
      await repository.store(dataContract);

      const transaction = repository.createTransaction();

      transaction.start();

      const storedDataContract = await repository.fetch(dataContract.getId(), transaction);

      expect(storedDataContract.toObject()).to.deep.equal(dataContract.toObject());
    });

    it('should return null if data contract not found', async () => {
      const storedDataContract = await repository.fetch(Buffer.alloc(32));

      expect(storedDataContract).to.equal(null);
    });
  });
});
