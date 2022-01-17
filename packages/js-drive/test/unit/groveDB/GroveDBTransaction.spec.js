const GroveDBTransaction = require('../../../lib/groveDB/GroveDBTransaction');
const GroveDBTransactionWrapper = require('../../../lib/groveDB/GroveDBInMemoryDecorator');

const GroveDBTransactionIsNotStartedError = require('../../../lib/groveDB/errors/GroveDBTransactionIsNotStartedError');
const GroveDBTransactionIsAlreadyStartedError = require('../../../lib/groveDB/errors/GroveDBTransactionIsAlreadyStartedError');

describe('GroveDBTransaction', () => {
  let groveDBTransaction;

  beforeEach(() => {
    const groveDBMock = {};

    groveDBTransaction = new GroveDBTransaction(groveDBMock);
  });

  describe('#start', () => {
    it('should start transaction', async () => {
      await groveDBTransaction.start();

      expect(groveDBTransaction.db).to.be.instanceOf(GroveDBTransactionWrapper);
    });

    it('should throw GroveDBTransactionIsAlreadyStartedError if transaction was started twice', async () => {
      await groveDBTransaction.start();

      try {
        await groveDBTransaction.start();

        expect.fail('Should throw an GroveDBTransactionIsAlreadyStartedError error');
      } catch (e) {
        expect(e).to.be.instanceOf(GroveDBTransactionIsAlreadyStartedError);
      }
    });
  });

  describe('#commit', () => {
    it('should commit transaction', async function it() {
      const persist = this.sinon.stub();

      groveDBTransaction.db = {
        persist,
      };

      const result = await groveDBTransaction.commit();

      expect(result).to.be.instanceOf(Object);
      expect(persist).to.be.calledOnce();
    });

    it('should throw GroveDBTransactionIsNotStartedError if transaction is not started', async () => {
      try {
        await groveDBTransaction.commit();

        expect.fail('Should throw an GroveDBTransactionIsNotStartedError error');
      } catch (e) {
        expect(e).to.be.instanceOf(GroveDBTransactionIsNotStartedError);
      }
    });
  });

  describe('#abort', () => {
    it('should abort transaction', async function it() {
      const reset = this.sinon.stub();

      groveDBTransaction.db = {
        reset,
      };

      const result = await groveDBTransaction.abort();

      expect(result).to.be.instanceOf(Object);
      expect(reset).to.be.calledOnce();
    });

    it('should throw GroveDBTransactionIsNotStartedError if transaction is not started', async () => {
      try {
        await groveDBTransaction.abort();

        expect.fail('should throw GroveDBTransactionIsNotStartedError');
      } catch (e) {
        expect(e).to.be.an.instanceof(GroveDBTransactionIsNotStartedError);
      }
    });
  });

  describe('#isStarted', () => {
    it('should return true if transaction is started', async () => {
      expect(groveDBTransaction.isStarted()).to.be.false();

      await groveDBTransaction.start();

      expect(groveDBTransaction.isStarted()).to.be.true();
    });

    it('should return false if transaction is aborted', async () => {
      expect(groveDBTransaction.isStarted()).to.be.false();

      await groveDBTransaction.start();

      expect(groveDBTransaction.isStarted()).to.be.true();

      await groveDBTransaction.abort();

      expect(groveDBTransaction.isStarted()).to.be.false();
    });
  });

  describe('#toObject', () => {
    it('should throw GroveDBTransactionIsNotStartedError if transaction is not started', () => {
      try {
        groveDBTransaction.toObject();

        expect.fail('should throw GroveDBTransactionIsNotStartedError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(GroveDBTransactionIsNotStartedError);
      }
    });

    it('should return all operations as plain object', async () => {
      await groveDBTransaction.start();

      const dataMap = new Map();
      dataMap.set('dataKey', 'dataValue');

      const deletedMap = new Map();
      deletedMap.set('deletedKey', 'deletedValue');

      groveDBTransaction.db.data = dataMap;
      groveDBTransaction.db.deleted = deletedMap;

      const result = groveDBTransaction.toObject();

      expect(result).to.deep.equal({
        updates: { dataKey: 'dataValue' },
        deletes: { deletedKey: 'deletedValue' },
      });
    });
  });

  describe('#populateFromObject', () => {
    it('should populate operations from plain object', async () => {
      const plainObject = {
        updates: { dataKey: 'dataValue' },
        deletes: { deletedKey: 'deletedValue' },
      };

      await GroveDbTransaction.start();

      GroveDbTransaction.populateFromObject(plainObject);

      expect(GroveDbTransaction.db.data.get('dataKey')).to.equal('dataValue');
      expect(GroveDbTransaction.db.deleted.get('deletedKey')).to.equal('deletedValue');
    });
  });
});
