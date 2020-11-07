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

      merkDbInMemoryDecorator.data.set(key.toString(MerkDbInMemoryDecorator.KEY_ENCODING), value);

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

    it('should throw "key not found" if value was deleted in transaction', () => {
      const key = Buffer.from([1, 2, 3]);
      const value = null;

      merkDBMock.getSync.returns(value);

      merkDbInMemoryDecorator.deleted.add(key.toString(MerkDbInMemoryDecorator.KEY_ENCODING));

      try {
        merkDbInMemoryDecorator.getSync(key);

        expect.fail('should throw "key not found" error');
      } catch (e) {
        expect(e.message).to.startsWith('key not found');
      }
    });
  });

  describe('#put', () => {
    it('should put value into transaction', () => {
      const key = Buffer.from([1, 2, 3]);
      const value = Buffer.from([4, 2]);

      const keyString = key.toString(MerkDbInMemoryDecorator.KEY_ENCODING);

      merkDbInMemoryDecorator.deleted.add(keyString);

      merkDbInMemoryDecorator.put(key, value);

      expect(merkDbInMemoryDecorator.deleted.has(keyString)).to.be.false();
      expect(merkDbInMemoryDecorator.data.get(keyString)).to.deep.equals(
        value,
      );
    });
  });

  describe('#delete', () => {
    it('should delete key from transaction', () => {
      const key = Buffer.from([1, 2, 3]);
      const value = Buffer.from([4, 2]);

      const keyString = key.toString(MerkDbInMemoryDecorator.KEY_ENCODING);

      merkDbInMemoryDecorator.data.set(keyString, value);

      merkDbInMemoryDecorator.delete(key);

      expect(merkDbInMemoryDecorator.deleted.has(keyString)).to.be.true();
      expect(merkDbInMemoryDecorator.data.get(keyString)).to.be.undefined();
      expect(merkDBMock.getSync).to.be.calledOnce();
    });

    it('should not add removing key to transaction if key not exists in merkDB', async () => {
      const error = new Error('key not found');

      const key = Buffer.from([1, 2, 3]);

      const keyString = key.toString(MerkDbInMemoryDecorator.KEY_ENCODING);

      merkDBMock.getSync.throws(error);

      merkDbInMemoryDecorator.delete(key);

      expect(merkDbInMemoryDecorator.deleted.has(keyString)).to.be.false();
      expect(merkDbInMemoryDecorator.data.get(keyString)).to.be.undefined();
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
      const keysToRemove = [
        Buffer.from([3, 3, 3]),
        Buffer.from([1, 1, 1]),
        Buffer.from([2, 2, 2]),
      ];
      const keysToAdd = [
        Buffer.from([6, 6, 6]),
        Buffer.from([4, 4, 4]),
        Buffer.from([5, 5, 5]),
      ];
      const valuesToAdd = [
        Buffer.from([1, 1]),
        Buffer.from([2, 2]),
        Buffer.from([3, 3]),
      ];

      keysToRemove.forEach((keyToRemove) => {
        merkDbInMemoryDecorator.deleted.add(keyToRemove.toString('hex'));
      });

      merkDbInMemoryDecorator.data.set(keysToAdd[0].toString('hex'), valuesToAdd[0]);
      merkDbInMemoryDecorator.data.set(keysToAdd[1].toString('hex'), valuesToAdd[1]);
      merkDbInMemoryDecorator.data.set(keysToAdd[2].toString('hex'), valuesToAdd[2]);

      merkDbInMemoryDecorator.persist();

      expect(merkDbInMemoryDecorator.data.size).to.be.equal(0);
      expect(merkDbInMemoryDecorator.deleted.size).to.be.equal(0);

      expect(merkDBMock.batch).to.be.calledOnce();
      expect(batchMock.put).to.be.calledThrice();
      expect(batchMock.delete).to.be.calledThrice();
      expect(batchMock.commitSync).to.be.calledOnce();

      expect(batchMock.put.getCall(0).args).to.have.deep.members([keysToAdd[1], valuesToAdd[1]]);
      expect(batchMock.put.getCall(1).args).to.have.deep.members([keysToAdd[2], valuesToAdd[2]]);
      expect(batchMock.put.getCall(2).args).to.have.deep.members([keysToAdd[0], valuesToAdd[0]]);

      expect(batchMock.delete.getCall(0).args).to.have.deep.members([keysToRemove[1]]);
      expect(batchMock.delete.getCall(1).args).to.have.deep.members([keysToRemove[2]]);
      expect(batchMock.delete.getCall(2).args).to.have.deep.members([keysToRemove[0]]);
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

      merkDbInMemoryDecorator.deleted.add(key.toString(MerkDbInMemoryDecorator.KEY_ENCODING));
      merkDbInMemoryDecorator.data.set(key.toString(MerkDbInMemoryDecorator.KEY_ENCODING), value);

      merkDbInMemoryDecorator.reset();

      expect(merkDbInMemoryDecorator.deleted.size).to.equal(0);
      expect(merkDbInMemoryDecorator.data.size).to.equal(0);
    });
  });
});
