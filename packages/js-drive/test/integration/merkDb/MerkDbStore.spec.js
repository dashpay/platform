const rimraf = require('rimraf');
const merk = require('merk');

const MerkDbStore = require('../../../lib/merkDb/MerkDbStore');

const MerkDbTransaction = require('../../../lib/merkDb/MerkDbTransaction');

describe('MerkDbStore', () => {
  let dbPath;
  let merkDb;
  let store;
  let key;
  let value;

  beforeEach(() => {
    dbPath = './db/merkdb-test';
    merkDb = merk(`${dbPath}/${Math.random()}`);

    store = new MerkDbStore(merkDb);

    key = Buffer.alloc(32).fill(1);
    value = Buffer.alloc(32).fill(2);
  });

  after(async () => {
    rimraf.sync(dbPath);
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
});
