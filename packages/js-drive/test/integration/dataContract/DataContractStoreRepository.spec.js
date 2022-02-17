const rimraf = require('rimraf');
const Drive = require('@dashevo/rs-drive');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const DataContractStoreRepository = require('../../../lib/dataContract/DataContractStoreRepository');

describe('DataContractStoreRepository', () => {
  let rsDrive;
  let store;
  let repository;
  let decodeProtocolEntity;
  let dataContract;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, 'testStore');

    decodeProtocolEntity = decodeProtocolEntityFactory();

    repository = new DataContractStoreRepository(store, decodeProtocolEntity);

    dataContract = getDataContractFixture();
  });

  afterEach(async () => {
    await rsDrive.close();
    rimraf.sync('./db/grovedb_test');
  });


  describe('#store', () => {
    beforeEach(async () => {
      await store.createTree([], DataContractStoreRepository.TREE_PATH[0]);
    });

    it('should store Data Contract', async () => {
      await repository.store(
        dataContract,
      );

      const encodedDataContract = await store.get(
        DataContractStoreRepository.TREE_PATH.concat([dataContract.getId().toBuffer()]),
        DataContractStoreRepository.DATA_CONTRACT_KEY,
      );

      const [protocolVersion, rawDataContract] = decodeProtocolEntity(encodedDataContract);

      rawDataContract.protocolVersion = protocolVersion;

      expect(dataContract.toJSON()).to.deep.equal(new DataContract(rawDataContract).toJSON());
    });

    it('should store Data Contract using transaction', async () => {
      await store.startTransaction();

      await repository.store(
        dataContract,
        true,
      );

      try {
        await store.get(
          DataContractStoreRepository.TREE_PATH.concat([dataContract.getId().toBuffer()]),
          DataContractStoreRepository.DATA_CONTRACT_KEY,
          { useTransaction: false },
        );

        expect.fail('should throw an error');
      } catch (e) {
        expect(e.message.startsWith('path not found')).to.be.true();
      }

      const dataFromTransaction = await store.get(
        DataContractStoreRepository.TREE_PATH.concat([dataContract.getId().toBuffer()]),
        DataContractStoreRepository.DATA_CONTRACT_KEY,
        { useTransaction: true },
      );

      let [protocolVersion, rawDataContract] = decodeProtocolEntity(dataFromTransaction);

      rawDataContract.protocolVersion = protocolVersion;

      expect(dataContract.toJSON()).to.deep.equal(new DataContract(rawDataContract).toJSON());

      await store.commitTransaction();

      const committedData = await store.get(
        DataContractStoreRepository.TREE_PATH.concat([dataContract.getId().toBuffer()]),
        DataContractStoreRepository.DATA_CONTRACT_KEY,
        { useTransaction: true },
      );

      [protocolVersion, rawDataContract] = decodeProtocolEntity(committedData);

      rawDataContract.protocolVersion = protocolVersion;

      expect(dataContract.toJSON()).to.deep.equal(new DataContract(rawDataContract).toJSON());
    });
  });

  describe('#fetch', () => {
    beforeEach(async () => {
      await store.createTree([], DataContractStoreRepository.TREE_PATH[0]);
    });

    it('should should fetch null if Data Contract not found', async () => {
      const storedDataContract = await repository.fetch(dataContract.getId());

      expect(storedDataContract).to.be.null();
    });

    it('should fetch Data Contract', async () => {
      await store.getDrive().applyContract(dataContract, false);

      const storedDataContract = await repository.fetch(dataContract.getId());

      expect(storedDataContract).to.be.an.instanceof(DataContract);
      expect(storedDataContract.toJSON()).to.deep.equal(dataContract.toJSON());
    });

    it('should fetch Data Contract using transaction', async () => {
      await store.startTransaction();

      await store.getDrive().applyContract(dataContract, true);

      const notFoundDataContract = await repository.fetch(dataContract.getId(), false);

      expect(notFoundDataContract).to.be.null();

      const transactionalDataContract = await repository.fetch(dataContract.getId(), true);

      expect(transactionalDataContract).to.be.an.instanceof(DataContract);
      expect(transactionalDataContract.toJSON()).to.deep.equal(dataContract.toJSON());

      await store.commitTransaction();

      const storedDataContract = await repository.fetch(dataContract.getId());

      expect(storedDataContract).to.be.an.instanceof(DataContract);
      expect(storedDataContract.toJSON()).to.deep.equal(dataContract.toJSON());
    });
  });

  describe('#createTree', () => {
    it('should create a tree', async () => {
      const result = await repository.createTree();

      expect(result).to.equal(repository);

      const data = await store.db.get(
        [],
        DataContractStoreRepository.TREE_PATH[0],
      );

      expect(data).to.deep.equal({
        type: 'tree',
        value: Buffer.alloc(32),
      });
    });
  });
});
