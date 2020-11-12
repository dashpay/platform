const Merk = require('@dashevo/merk');

const MerkDbStore = require('../../../lib/merkDb/MerkDbStore');

const MerkDbTransaction = require('../../../lib/merkDb/MerkDbTransaction');

describe('MerkDbStore', () => {
  let merkDb;
  let store;
  let key;
  let value;

  beforeEach(() => {
    merkDb = new Merk('./db/merkdb-test');

    store = new MerkDbStore(merkDb);

    key = Buffer.alloc(32).fill(1);
    value = Buffer.alloc(32).fill(2);
  });

  afterEach(async () => {
    merkDb.close();
    merkDb.destroy();
  });

  describe('#put', () => {
    it('should store value', () => {
      const result = store.put(key, value);

      expect(result).to.be.instanceOf(MerkDbStore);

      const actualValue = merkDb.getSync(key);

      expect(actualValue).to.be.deep.equal(value);
    });

    it('should store value in transaction', async () => {
      const transaction = store.createTransaction();

      expect(transaction).to.be.instanceOf(MerkDbTransaction);

      transaction.start();

      // store data in transaction
      store.put(key, value, transaction);

      // check we don't have data in db before commit
      try {
        merkDb.getSync(key);

        expect.fail('Should fail with NotFoundError error');
      } catch (e) {
        expect(e.message.startsWith('key not found')).to.be.true();
      }

      // check we can't fetch data without transaction
      const notFoundValue = store.get(key);

      expect(notFoundValue).to.be.null();

      // check we can fetch data inside transaction
      const valueFromTransaction = store.get(key, transaction);

      expect(valueFromTransaction).to.deep.equal(value);

      await transaction.commit();

      // check we have data in db after commit
      const storedValue = merkDb.getSync(key);

      expect(storedValue).to.deep.equal(value);
    });
  });

  describe('#get', () => {
    it('should return null if key was not found', () => {
      const result = store.get(key);

      expect(result).to.be.null();
    });

    it('should return stored value', () => {
      merkDb.batch().put(key, value).commitSync();

      const result = store.get(key);

      expect(result).to.deep.equal(value);
    });

    it('should return stored value with transaction', () => {
      store.put(key, value);

      const transaction = store.createTransaction();

      transaction.start();

      const result = store.get(key, transaction);

      expect(result).to.deep.equal(value);
    });
  });

  describe('#delete', () => {
    beforeEach(() => {
      merkDb.batch()
        .put(key, value)
        // MerkDB doesn't delete the last key for some reason
        // So we need to add an extra one to test delete functionality
        // on empty database
        .put(Buffer.alloc(1), Buffer.alloc(1))
        .commitSync();
    });

    it('should delete value', () => {
      store.delete(key);

      try {
        merkDb.getSync(key);

        expect.fail('should throw key not found error');
      } catch (e) {
        expect(e.message).to.startsWith('key not found');
      }
    });

    it('should delete value in transaction', async () => {
      const transaction = store.createTransaction();

      expect(transaction).to.be.instanceOf(MerkDbTransaction);

      transaction.start();

      // Delete a value from transaction
      store.delete(key, transaction);

      // Now it should be absent there
      const valueFromTransaction = store.get(key, transaction);
      expect(valueFromTransaction).to.be.null();

      // But should be still present in store
      const valueFromStore = store.get(key);
      expect(valueFromStore).to.deep.equal(value);

      await transaction.commit();

      // When we commit transaction this key should disappear from store too
      const valueFromStoreAfterCommit = store.get(key);
      expect(valueFromStoreAfterCommit).to.be.null();
    });
  });

  describe('#getRootHash', () => {
    it('should return a empty hash for empty store', () => {
      const result = store.getRootHash();

      expect(result).to.deep.equal(Buffer.alloc(20));
    });

    it('should return a root hash for store with value', () => {
      merkDb.batch()
        .put(key, value)
        .commitSync();

      const valueHash = Buffer.from('8431f62d3d7b7f16ec98321019d0e65ad191bea1', 'hex');

      const result = store.getRootHash();

      expect(result).to.deep.equal(valueHash);
    });
  });

  describe('#createTransaction', () => {
    it('should create a transaction', () => {
      const result = store.createTransaction();

      expect(result).to.be.instanceOf(MerkDbTransaction);
    });
  });

  describe('#getProof', () => {
    beforeEach(() => {
      merkDb.batch()
        .put(key, value)
        .put(Buffer.alloc(1), Buffer.alloc(1))
        .commitSync();
    });

    it('should return a proof', async () => {
      const result = store.getProof([key]);

      expect(result).to.be.deep.equal(
        Buffer.from('0119fa9955a06bc5cd46918709596e488ba2e8d96d032001010101010101010101010101010101010101010101010101010101010101012000020202020202020202020202020202020202020202020202020202020202020210', 'hex'),
      );
    });
  });
});
