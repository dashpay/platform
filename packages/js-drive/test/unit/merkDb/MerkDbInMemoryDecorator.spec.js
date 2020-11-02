const MerkDbInMemoryDecorator = require('../../../lib/merkDb/MerkDbInMemoryDecorator');

describe('MerkDbInMemoryDecorator', () => {
  let merkDbInMemoryDecorator;
  let merkDBMock;
  let batchMock;

  beforeEach(function beforeEach() {
    batchMock = {
      put: this.sinon.stub(),
      delete: this.sinon.stub(),
      commitSync: this.sinon.stub(),
    };

    batchMock.put.returns(batchMock);
    batchMock.delete.returns(batchMock);
    batchMock.commitSync.returns(batchMock);

    merkDBMock = {
      getSync: this.sinon.stub(),
      batch: this.sinon.stub().returns(batchMock),
    };

    merkDbInMemoryDecorator = new MerkDbInMemoryDecorator(merkDBMock);
  });

  describe('#getSync', () => {
    it('should return value from transaction', () => {
      const key = Buffer.from([1, 2, 3]);
      const value = Buffer.from([4, 2]);

      merkDbInMemoryDecorator.data.set(key.toString('hex'), value);

      const result = merkDbInMemoryDecorator.getSync(key);

      expect(result).to.deep.equal(value);
    });

    it('should return value from db', () => {
      const key = Buffer.from([1, 2, 3]);
      const value = Buffer.from([4, 2]);

      merkDBMock.getSync.returns(value);

      const result = merkDbInMemoryDecorator.getSync(key);

      expect(result).to.deep.equal(value);
    });

    it('should return null if value was removed in transaction', () => {
      const key = Buffer.from([1, 2, 3]);
      const value = null;

      merkDBMock.getSync.returns(value);

      merkDbInMemoryDecorator.deleted.add(key.toString('hex'));

      const result = merkDbInMemoryDecorator.getSync(key);

      expect(result).to.deep.equal(value);
    });
  });

  describe('#put', () => {
    it('should put value into transaction', () => {
      const key = Buffer.from([1, 2, 3]);
      const value = Buffer.from([4, 2]);

      merkDbInMemoryDecorator.deleted.add(key.toString('hex'));

      merkDbInMemoryDecorator.put(key, value);

      expect(merkDbInMemoryDecorator.deleted.has(key.toString('hex'))).to.be.false();
      expect(merkDbInMemoryDecorator.data.get(key.toString('hex'))).to.deep.equals(
        value,
      );
    });
  });

  describe('#delete', () => {
    it('should delete key from transaction', () => {
      const key = Buffer.from([1, 2, 3]);
      const value = Buffer.from([4, 2]);

      merkDbInMemoryDecorator.data.set(key.toString('hex'), value);

      merkDbInMemoryDecorator.delete(key);

      expect(merkDbInMemoryDecorator.deleted.has(key.toString('hex'))).to.be.true();
      expect(merkDbInMemoryDecorator.data.get(key.toString('hex'))).to.be.undefined();
      expect(merkDBMock.getSync).to.be.calledOnce();
    });

    it('should not add removing key to transaction if key not exists in merkDB', async () => {
      const error = new Error('key not found');

      const key = Buffer.from([1, 2, 3]);

      merkDBMock.getSync.throws(error);

      merkDbInMemoryDecorator.delete(key);

      expect(merkDbInMemoryDecorator.deleted.has(key.toString('hex'))).to.be.false();
      expect(merkDbInMemoryDecorator.data.get(key.toString('hex'))).to.be.undefined();
      expect(merkDBMock.getSync).to.be.calledOnce();
    });

    it('should throw an error', async () => {
      const error = new Error('unknown error');

      const key = Buffer.from([1, 2, 3]);

      merkDBMock.getSync.throws(error);

      try {
        merkDbInMemoryDecorator.delete(key);

        expect.fail('should throw unknown error');
      } catch (e) {
        expect(e).to.equal(error);
      }
    });
  });

  describe('#persist', () => {
    it('should persist in memory data to merk db', () => {
      const keyToAdd = Buffer.from([1, 2, 3]);
      const keyToRemove = Buffer.from([1, 2, 3]);
      const valueToAdd = Buffer.from([4, 2]);

      merkDbInMemoryDecorator.data.set(keyToAdd.toString('hex'), valueToAdd);
      merkDbInMemoryDecorator.deleted.add(keyToRemove.toString('hex'));

      merkDbInMemoryDecorator.persist();

      expect(merkDbInMemoryDecorator.data.size).to.be.equal(0);
      expect(merkDbInMemoryDecorator.deleted.size).to.be.equal(0);

      expect(merkDBMock.batch).to.be.calledOnce();
      expect(batchMock.put).to.be.calledOnce();
      expect(batchMock.delete).to.be.calledOnce();
      expect(batchMock.commitSync).to.be.calledOnce();
    });

    it('should skip persisting if nothing to persist', async () => {
      expect(merkDbInMemoryDecorator.data.size).to.be.equal(0);
      expect(merkDbInMemoryDecorator.deleted.size).to.be.equal(0);

      merkDbInMemoryDecorator.persist();

      expect(merkDbInMemoryDecorator.data.size).to.be.equal(0);
      expect(merkDbInMemoryDecorator.deleted.size).to.be.equal(0);

      expect(merkDBMock.batch).to.be.not.called();
      expect(batchMock.put).to.be.not.called();
      expect(batchMock.delete).to.be.not.called();
      expect(batchMock.commitSync).to.be.not.called();
    });
  });

  describe('#reset', () => {
    it('should reset in memory data', () => {
      const key = Buffer.from([1, 2, 3]);
      const value = Buffer.from([4, 2]);

      merkDbInMemoryDecorator.deleted.add(key.toString('hex'));
      merkDbInMemoryDecorator.data.set(key.toString('hex'), value);

      merkDbInMemoryDecorator.reset();

      expect(merkDbInMemoryDecorator.deleted.size).to.equal(0);
      expect(merkDbInMemoryDecorator.data.size).to.equal(0);
    });
  });
});
