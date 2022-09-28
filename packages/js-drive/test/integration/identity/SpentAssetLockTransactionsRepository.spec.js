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

    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, logger);

    repository = new SpentAssetLockTransactionsRepository(store);

    await store.createTree([], SpentAssetLockTransactionsRepository.TREE_PATH[0]);
  });

  afterEach(async () => {
    await rsDrive.close();
    fs.rmSync('./db/grovedb_test', { recursive: true });
  });

  describe('#store', () => {
    it('should store outpoint', async () => {
      const result = await repository.store(outPointBuffer, {
        useTransaction: true,
      });

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

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
      expect(result.getOperations().length).to.be.greaterThan(0);

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
      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.be.deep.equal(Buffer.from([0]));
    });
  });
});
