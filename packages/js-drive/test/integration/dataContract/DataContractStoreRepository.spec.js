const rimraf = require('rimraf');
const Drive = require('@dashevo/rs-drive');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const DataContractStoreRepository = require('../../../lib/dataContract/DataContractStoreRepository');
const noopLogger = require('../../../lib/util/noopLogger');
const StorageResult = require('../../../lib/storage/StorageResult');

describe('DataContractStoreRepository', () => {
  let rsDrive;
  let store;
  let repository;
  let decodeProtocolEntity;
  let dataContract;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, noopLogger);

    decodeProtocolEntity = decodeProtocolEntityFactory();

    repository = new DataContractStoreRepository(store, decodeProtocolEntity, noopLogger);

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
      const result = await repository.store(
        dataContract,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const encodedDataContractResult = await store.get(
        DataContractStoreRepository.TREE_PATH.concat([dataContract.getId().toBuffer()]),
        DataContractStoreRepository.DATA_CONTRACT_KEY,
      );

      const [protocolVersion, rawDataContract] = decodeProtocolEntity(
        encodedDataContractResult.getValue(),
      );

      rawDataContract.protocolVersion = protocolVersion;

      const fetchedDataContract = new DataContract(rawDataContract);

      expect(dataContract.toObject()).to.deep.equal(fetchedDataContract.toObject());
    });

    it('should store Data Contract using transaction', async () => {
      await store.startTransaction();

      const result = await repository.store(
        dataContract,
        { useTransaction: true },
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const notFoundDataContractResult = await store.get(
        DataContractStoreRepository.TREE_PATH.concat([dataContract.getId().toBuffer()]),
        DataContractStoreRepository.DATA_CONTRACT_KEY,
        { useTransaction: false },
      );

      expect(notFoundDataContractResult.getValue()).to.be.null();

      const dataFromTransactionResult = await store.get(
        DataContractStoreRepository.TREE_PATH.concat([dataContract.getId().toBuffer()]),
        DataContractStoreRepository.DATA_CONTRACT_KEY,
        { useTransaction: true },
      );

      let [protocolVersion, rawDataContract] = decodeProtocolEntity(
        dataFromTransactionResult.getValue(),
      );

      rawDataContract.protocolVersion = protocolVersion;

      const fetchedDataContract = new DataContract(rawDataContract);

      expect(dataContract.toObject()).to.deep.equal(fetchedDataContract.toObject());

      await store.commitTransaction();

      const committedDataResult = await store.get(
        DataContractStoreRepository.TREE_PATH.concat([dataContract.getId().toBuffer()]),
        DataContractStoreRepository.DATA_CONTRACT_KEY,
        { useTransaction: true },
      );

      [protocolVersion, rawDataContract] = decodeProtocolEntity(committedDataResult.getValue());

      rawDataContract.protocolVersion = protocolVersion;

      const fetchedOneMoreDataContract = new DataContract(rawDataContract);

      expect(dataContract.toObject()).to.deep.equal(fetchedOneMoreDataContract.toObject());
    });

    it('should not store Data Contract with dry run', async () => {
      const result = await repository.store(
        dataContract,
        { dryRun: true },
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const encodedDataContractResult = await store.get(
        DataContractStoreRepository.TREE_PATH.concat([dataContract.getId().toBuffer()]),
        DataContractStoreRepository.DATA_CONTRACT_KEY,
      );

      expect(encodedDataContractResult.getValue()).to.be.null();
    });
  });

  describe('#fetch', () => {
    beforeEach(async () => {
      await store.createTree([], DataContractStoreRepository.TREE_PATH[0]);
    });

    it('should should fetch null if Data Contract not found', async () => {
      const result = await repository.fetch(dataContract.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.be.null();
    });

    it('should fetch Data Contract', async () => {
      await store.getDrive().applyContract(dataContract, new Date(), false);

      const result = await repository.fetch(dataContract.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const storedDataContract = result.getValue();

      expect(storedDataContract).to.be.an.instanceof(DataContract);
      expect(storedDataContract.toObject()).to.deep.equal(storedDataContract.toObject());
    });

    it('should fetch Data Contract using transaction', async () => {
      await store.startTransaction();

      await store.getDrive().applyContract(dataContract, new Date(), true);

      const notFoundDataContractResult = await repository.fetch(dataContract.getId(), {
        useTransaction: false,
      });

      expect(notFoundDataContractResult.getValue()).to.be.null();

      const transactionalDataContractResult = await repository.fetch(dataContract.getId(), {
        useTransaction: true,
      });

      const transactionalDataContract = transactionalDataContractResult.getValue();

      expect(transactionalDataContract).to.be.an.instanceof(DataContract);
      expect(transactionalDataContract.toObject()).to.deep.equal(dataContract.toObject());

      await store.commitTransaction();

      const storedDataContractResult = await repository.fetch(dataContract.getId());

      const storedDataContract = storedDataContractResult.getValue();

      expect(storedDataContract).to.be.an.instanceof(DataContract);
      expect(storedDataContract.toObject()).to.deep.equal(dataContract.toObject());
    });

    it('should fetch null on dry run', async () => {
      await store.getDrive().applyContract(dataContract, new Date(), false);

      const result = await repository.fetch(dataContract.getId(), { dryRun: true });

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.be.null();
    });
  });

  describe('#createTree', () => {
    it('should create a tree', async () => {
      const result = await repository.createTree();

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

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

  describe('#prove', () => {
    beforeEach(async () => {
      await store.createTree([], DataContractStoreRepository.TREE_PATH[0]);
    });

    it('should should return null if Data Contract not found', async () => {
      const result = await repository.prove(dataContract.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.be.null();
    });

    it('should return proof', async () => {
      await store.getDrive().applyContract(dataContract, new Date(), false);

      const result = await repository.prove(dataContract.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const proof = result.getValue();

      expect(proof).to.be.an.instanceof(Buffer);
    });

    it.skip('should return proof using transaction', async () => {
      await store.startTransaction();

      await store.getDrive().applyContract(dataContract, new Date(), true);

      const notFoundDataContractResult = await repository.prove(dataContract.getId(), {
        useTransaction: false,
      });

      expect(notFoundDataContractResult.getValue()).to.be.null();

      const transactionalDataContractResult = await repository.prove(dataContract.getId(), {
        useTransaction: true,
      });

      const transactionalDataContract = transactionalDataContractResult.getValue();

      expect(transactionalDataContract).to.be.an.instanceof(Buffer);

      await store.commitTransaction();

      const storedDataContractResult = await repository.prove(dataContract.getId());

      const storedDataContract = storedDataContractResult.getValue();

      expect(storedDataContract).to.be.an.instanceof(Buffer);
    });
  });

  describe('#proveMany', () => {
    let dataContract2;

    beforeEach(async () => {
      dataContract2 = new DataContract({
        $id: generateRandomIdentifier().toBuffer(),
        ownerId: generateRandomIdentifier().toBuffer(),
        contractId: generateRandomIdentifier().toBuffer(),
        documents: {
          niceDocument: {
            properties: {
              nice: {
                type: 'boolean',
              },
            },
          },
        },
      });

      await store.createTree([], DataContractStoreRepository.TREE_PATH[0]);
    });

    it('should should return null if Data Contract not found', async () => {
      const result = await repository.proveMany([dataContract.getId(), dataContract2.getId()]);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.be.null();
    });

    it('should return proof', async () => {
      await store.getDrive().applyContract(dataContract, new Date(), false);
      await store.getDrive().applyContract(dataContract2, new Date(), false);

      const result = await repository.proveMany([dataContract.getId(), dataContract2.getId()]);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const proof = result.getValue();

      expect(proof).to.be.an.instanceof(Buffer);
    });

    it.skip('should return proof using transaction', async () => {
      await store.startTransaction();

      await store.getDrive().applyContract(dataContract, new Date(), true);
      await store.getDrive().applyContract(dataContract2, new Date(), true);

      const notFoundDataContractResult = await repository.prove(
        [dataContract.getId(), dataContract2.getId()], {
          useTransaction: false,
        },
      );

      expect(notFoundDataContractResult.getValue()).to.be.null();

      const transactionalDataContractResult = await repository.proveMany(
        [dataContract.getId(), dataContract2.getId()],
        { useTransaction: true },
      );

      const transactionalDataContract = transactionalDataContractResult.getValue();

      expect(transactionalDataContract).to.be.an.instanceof(Buffer);

      await store.commitTransaction();

      const storedDataContractResult = await repository.proveMany(
        [dataContract.getId(), dataContract2.getId()],
      );

      const storedDataContract = storedDataContractResult.getValue();

      expect(storedDataContract).to.be.an.instanceof(Buffer);
    });
  });
});
