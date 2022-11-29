const rimraf = require('rimraf');

const Drive = require('@dashevo/rs-drive');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const logger = require('../../../lib/util/noopLogger');
const StorageResult = require('../../../lib/storage/StorageResult');

describe('GroveDBStore', () => {
  let rsDrive;
  let store;
  let key;
  let value;
  let testTreePath;
  let otherTreePath;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test', {
      dataContractsGlobalCacheSize: 500,
      dataContractsBlockCacheSize: 500,
    });

    store = new GroveDBStore(rsDrive, logger);

    testTreePath = [Buffer.from('testTree')];
    otherTreePath = [Buffer.from('otherTree')];

    await store.createTree([], testTreePath[0]);
    await store.createTree([], otherTreePath[0]);

    key = Buffer.alloc(32).fill(1);
    value = Buffer.alloc(32).fill(2);
  });

  afterEach(async () => {
    await rsDrive.close();
    rimraf.sync('./db/grovedb_test');
  });

  describe('#put', () => {
    it('should store value', async () => {
      const result = await store.put(testTreePath, key, value);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const actualValue = await rsDrive.getGroveDB().get(testTreePath, key);

      expect(actualValue).to.be.deep.equal({
        type: 'item',
        value,
      });
    });

    it('should store value in transaction', async () => {
      await store.startTransaction();

      // store data in transaction
      await store.put(testTreePath, key, value, {
        useTransaction: true,
      });

      // check we don't have data in db before commit
      try {
        await rsDrive.getGroveDB().get(testTreePath, key);

        expect.fail('Should fail with NotFoundError error');
      } catch (e) {
        expect(e.message.startsWith('grovedb: path key not found: key not found in Merk')).to.be.true();
      }

      // check we can't fetch data without transaction
      const notFoundValueResult = await store.get(testTreePath, key);

      expect(notFoundValueResult.getValue()).to.be.null();

      // check we can fetch data inside transaction
      const valueFromTransactionResult = await store.get(testTreePath, key, {
        useTransaction: true,
      });

      expect(valueFromTransactionResult).to.be.instanceOf(StorageResult);
      expect(valueFromTransactionResult.getOperations().length).to.equal(0);

      expect(valueFromTransactionResult.getValue()).to.deep.equal(value);

      await store.commitTransaction();

      // check we have data in db after commit
      const storedValue = await rsDrive.getGroveDB().get(testTreePath, key);

      expect(storedValue).to.deep.equal({
        type: 'item',
        value,
      });
    });
  });

  describe('#get', () => {
    it('should return null if key was not found', async () => {
      const result = await store.get(testTreePath, key);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.be.null();
    });

    it('should return stored value', async () => {
      await rsDrive.getGroveDB().insert(
        testTreePath,
        key,
        { type: 'item', value },
        false,
      );

      const result = await store.get(testTreePath, key);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.deep.equal(value);
    });

    it('should return stored value with transaction', async () => {
      await store.put(testTreePath, key, value);

      await store.startTransaction();

      const result = await store.get(testTreePath, key, {
        useTransaction: true,
      });

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.deep.equal(value);
    });
  });

  describe('#putReference', () => {
    it('should put an item by reference', async () => {
      await store.put(otherTreePath, key, value);

      const result = await store.putReference(testTreePath, key, [otherTreePath[0], key]);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const getResult = await store.get(testTreePath, key);

      expect(getResult.getValue()).to.deep.equal(value);
    });

    it('should put an item by reference in transaction', async () => {
      await store.put(otherTreePath, key, value);

      await store.startTransaction();

      const result = await store.putReference(testTreePath, key, [otherTreePath[0], key], {
        useTransaction: true,
      });

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const nonTxResult = await store.get(testTreePath, key);

      expect(nonTxResult.getValue()).to.be.null();

      const txResult = await store.get(testTreePath, key, {
        useTransaction: true,
      });

      expect(txResult.getValue()).to.deep.equal(value);
    });
  });

  describe('#query', () => {
    it('should return results', async () => {
      await store.put(testTreePath, key, value);

      const result = await store.query({
        path: testTreePath,
        query: {
          query: {
            items: [
              {
                type: 'rangeFull',
              },
            ],
          },
        },
      });

      expect(result).to.have.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.have.lengthOf(1);

      const [item] = result.getValue();

      expect(item).to.deep.equal(value);
    });
  });

  describe('#proveQuery', () => {
    it('should return proof', async () => {
      await store.put(testTreePath, key, value);

      const result = await store.proveQuery({
        path: testTreePath,
        query: {
          query: {
            items: [
              {
                type: 'rangeFull',
              },
            ],
          },
        },
      });

      expect(result).to.have.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);
    });
  });

  describe('#delete', () => {
    it('should delete value', async () => {
      await store.put(testTreePath, key, value);

      const result = await store.delete(testTreePath, key);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      try {
        await rsDrive.getGroveDB().get(testTreePath, key);

        expect.fail('should throw no value found for key error');
      } catch (e) {
        expect(e.message.startsWith('grovedb: path key not found: key not found in Merk')).to.be.true();
      }
    });

    it('should delete value in transaction', async () => {
      await store.put(testTreePath, key, value);

      await store.startTransaction();

      // Delete a value from transaction
      const result = await store.delete(testTreePath, key, {
        useTransaction: true,
      });

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      // Now it should be absent there
      const valueFromTransactionResult = await store.get(testTreePath, key, {
        useTransaction: true,
      });

      expect(valueFromTransactionResult.getValue()).to.be.null();

      // But should be still present in store
      const valueFromStoreResult = await store.get(testTreePath, key);
      expect(valueFromStoreResult.getValue()).to.deep.equal(value);

      await store.commitTransaction();

      // When we commit transaction this key should disappear from store too
      const valueFromStoreAfterCommitResult = await store.get(testTreePath, key);
      expect(valueFromStoreAfterCommitResult.getValue()).to.be.null();
    });
  });

  describe('#getAux', () => {
    it('should get an auxiliary data from db', async () => {
      await rsDrive.getGroveDB().putAux(key, value);

      const result = await store.getAux(key);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.deep.equal(value);
    });

    it('should get an auxiliary data from db with transaction', async () => {
      await rsDrive.getGroveDB().putAux(key, value);

      await store.startTransaction();

      const result = await store.getAux(key, {
        useTransaction: true,
      });

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.deep.equal(value);
    });
  });

  describe('#putAux', () => {
    it('should put an auxiliary data', async () => {
      await store.putAux(key, value);

      const result = await rsDrive.getGroveDB().getAux(key);

      expect(result).to.deep.equal(value);
    });

    it('should put an auxiliary data using transaction', async () => {
      await store.startTransaction();

      await store.putAux(key, value, {
        useTransaction: true,
      });

      const nonTxResult = await rsDrive.getGroveDB().getAux(key);

      expect(nonTxResult).to.be.null();

      const txResult = await rsDrive.getGroveDB().getAux(key, true);

      expect(txResult).to.deep.equal(value);
    });
  });

  describe('#deleteAux', () => {
    it('should delete an auxiliary data', async () => {
      await store.putAux(key, value);

      const getResult = await store.getAux(key);

      expect(getResult.getValue()).to.deep.equal(value);

      const deleteResult = await store.deleteAux(key);

      expect(deleteResult).to.be.instanceOf(StorageResult);
      expect(deleteResult.getOperations().length).to.equal(0);

      const deletedValue = await rsDrive.getGroveDB().getAux(key);

      expect(deletedValue).to.be.null();
    });

    it('should delete an auxiliary data within transaction', async () => {
      await store.putAux(key, value);

      await store.startTransaction();

      const deleteResult = await store.deleteAux(key, {
        useTransaction: true,
      });

      expect(deleteResult).to.be.instanceOf(StorageResult);
      expect(deleteResult.getOperations().length).to.equal(0);

      const nonTxResult = await store.getAux(key);

      expect(nonTxResult.getValue()).to.deep.equal(value);

      const txResult = await store.getAux(key, {
        useTransaction: true,
      });

      expect(txResult.getValue()).to.be.null();
    });
  });

  describe('#getRootHash', () => {
    it('should return a null hash for empty store', async () => {
      await rsDrive.close();

      rimraf.sync('./db/grovedb_test');

      rsDrive = new Drive('./db/grovedb_test', {
        dataContractsGlobalCacheSize: 500,
        dataContractsBlockCacheSize: 500,
      });

      store = new GroveDBStore(rsDrive, logger);

      const result = await store.getRootHash();

      expect(result).to.deep.equal(Buffer.alloc(32).fill(0));
    });

    it('should return a root hash for store with value', async () => {
      await store.put(testTreePath, key, value);

      const valueHash = Buffer.from('9522321fe08ddbbd5a37cf875cdd7f7a104ac9b9e9246f1454b7360341b29124', 'hex');

      const result = await store.getRootHash();

      expect(result).to.deep.equal(valueHash);
    });
  });
});
