const Drive = require('@dashevo/rs-drive');
const fs = require('fs');

const SpentAssetLockTransactionsRepository = require('../../../lib/identity/SpentAssetLockTransactionsRepository');
const StorageResult = require('../../../lib/storage/StorageResult');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const logger = require('../../../lib/util/noopLogger');

describe('SpentAssetLockTransactionsRepository', () => {
  let outPointBuffer;
  let repository;
  let store;
  let rsDrive;

  beforeEach(async () => {
    outPointBuffer = Buffer.from([42]);

    rsDrive = new Drive('./db/grovedb_test', {
      dataContractsGlobalCacheSize: 500,
      dataContractsBlockCacheSize: 500,
    });

    store = new GroveDBStore(rsDrive, logger);

    await rsDrive.createInitialStateStructure();

    repository = new SpentAssetLockTransactionsRepository(store);
  });

  afterEach(async () => {
    await rsDrive.close();
    fs.rmSync('./db/grovedb_test', { recursive: true });
  });

  describe('#store', () => {
    it('should store outpoint', async () => {
      const result = await repository.store(outPointBuffer);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const placeholderResult = await store.get(
        SpentAssetLockTransactionsRepository.TREE_PATH,
        outPointBuffer,
      );

      expect(placeholderResult.getValue()).to.deep.equal(Buffer.from([0]));
    });
  });

  describe('#fetch', () => {
    it('should return null if outpoint is not present', async () => {
      const result = await repository.fetch(outPointBuffer);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.be.null();
    });

    it('should return buffer containing [0]', async () => {
      await store.put(
        SpentAssetLockTransactionsRepository.TREE_PATH,
        outPointBuffer,
        Buffer.from([0]),
      );

      const result = await repository.fetch(outPointBuffer);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.be.deep.equal(Buffer.from([0]));
    });
  });
});
