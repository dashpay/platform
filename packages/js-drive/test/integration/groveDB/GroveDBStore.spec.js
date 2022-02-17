const rimraf = require('rimraf');

const Drive = require('@dashevo/rs-drive/node/Drive');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');

describe('GroveDBStore', () => {
  let rsDrive;
  let store;
  let key;
  let value;
  let testTreePath;
  let otherTreePath;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');

    store = new GroveDBStore(rsDrive, 'testStore');

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

      expect(result).to.be.instanceOf(GroveDBStore);

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
        expect(e.message.indexOf('path key not found: key not found in Merk') !== -1).to.be.true();
      }

      // check we can't fetch data without transaction
      const notFoundValue = await store.get(testTreePath, key);

      expect(notFoundValue).to.be.null();

      // check we can fetch data inside transaction
      const valueFromTransaction = await store.get(testTreePath, key, {
        useTransaction: true,
      });

      expect(valueFromTransaction).to.deep.equal(value);

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

      expect(result).to.be.null();
    });

    it('should return stored value', async () => {
      await rsDrive.getGroveDB().insert(
        testTreePath,
        key,
        { type: 'item', value },
        false,
      );

      const result = await store.get(testTreePath, key);

      expect(result).to.deep.equal(value);
    });

    it('should return stored value with transaction', async () => {
      await store.put(testTreePath, key, value);

      await store.startTransaction();

      const result = await store.get(testTreePath, key, {
        useTransaction: true,
      });

      expect(result).to.deep.equal(value);
    });
  });

  describe('#putReference', () => {
    it('should put an item by reference', async () => {
      await store.put(otherTreePath, key, value);

      await store.putReference(testTreePath, key, [otherTreePath[0], key]);

      const result = await store.get(testTreePath, key);

      expect(result).to.deep.equal(value);
    });

    it('should put an item by reference in transaction', async () => {
      await store.put(otherTreePath, key, value);

      await store.startTransaction();

      await store.putReference(testTreePath, key, [otherTreePath[0], key], {
        useTransaction: true,
      });

      const nonTxResult = await store.get(testTreePath, key);

      expect(nonTxResult).to.be.null();

      const txResult = await store.get(testTreePath, key, {
        useTransaction: true,
      });

      expect(txResult).to.deep.equal(value);
    });
  });

  describe('#delete', () => {
    it('should delete value', async () => {
      await store.put(testTreePath, key, value);

      await store.delete(testTreePath, key);

      try {
        await rsDrive.getGroveDB().get(testTreePath, key);

        expect.fail('should throw no value found for key error');
      } catch (e) {
        expect(e.message.indexOf('path key not found: key not found in Merk')).to.not.equal(-1);
      }
    });

    it('should delete value in transaction', async () => {
      await store.put(testTreePath, key, value);

      await store.startTransaction();

      // Delete a value from transaction
      await store.delete(testTreePath, key, {
        useTransaction: true,
      });

      // Now it should be absent there
      const valueFromTransaction = await store.get(testTreePath, key, {
        useTransaction: true,
      });
      expect(valueFromTransaction).to.be.null();

      // But should be still present in store
      const valueFromStore = await store.get(testTreePath, key);
      expect(valueFromStore).to.deep.equal(value);

      await store.commitTransaction();

      // When we commit transaction this key should disappear from store too
      const valueFromStoreAfterCommit = await store.get(testTreePath, key);
      expect(valueFromStoreAfterCommit).to.be.null();
    });
  });

  describe('#getAux', () => {
    it('should get an auxiliary data from db', async () => {
      await rsDrive.getGroveDB().putAux(key, value);

      const result = await store.getAux(key);

      expect(result).to.deep.equal(value);
    });

    it('should get an auxiliary data from db with transaction', async () => {
      await rsDrive.getGroveDB().putAux(key, value);

      await store.startTransaction();

      const result = await store.getAux(key, {
        useTransaction: true,
      });

      expect(result).to.deep.equal(value);
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

      let result = await store.getAux(key);

      expect(result).to.deep.equal(value);

      await store.deleteAux(key);

      result = await rsDrive.getGroveDB().getAux(key);

      expect(result).to.be.null();
    });

    it('should delete an auxiliary data within transaction', async () => {
      await store.putAux(key, value);

      await store.startTransaction();

      await store.deleteAux(key, {
        useTransaction: true,
      });

      const nonTxResult = await store.getAux(key);

      expect(nonTxResult).to.deep.equal(value);

      const txResult = await store.getAux(key, {
        useTransaction: true,
      });

      expect(txResult).to.be.null();
    });
  });

  describe('#getRootHash', () => {
    it('should return a null hash for empty store', async () => {
      await rsDrive.close();

      rimraf.sync('./db/grovedb_test');

      rsDrive = new Drive('./db/grovedb_test');
      store = new GroveDBStore(rsDrive, 'testStore');

      const result = await store.getRootHash();

      expect(result).to.deep.equal(Buffer.alloc(32).fill(0));
    });

    it('should return a root hash for store with value', async () => {
      await store.put(testTreePath, key, value);

      const valueHash = Buffer.from('d11176d953464b9f4e49a454885ec3b2c89ce659a5e8ede23ea46dfbf737a2a5', 'hex');

      const result = await store.getRootHash();

      expect(result).to.deep.equal(valueHash);
    });
  });
});
