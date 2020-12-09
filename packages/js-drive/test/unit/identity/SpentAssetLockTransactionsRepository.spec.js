const StoreMock = require('../../../lib/test/mock/StoreMock');

const SpentAssetLockTransactionsRepository = require('../../../lib/identity/SpentAssetLockTransactionsRepository');

describe('SpentAssetLockTransactionsRepository', () => {
  let outPointBuffer;
  let repository;
  let storeMock;
  let transactionMock;

  beforeEach(function beforeEach() {
    outPointBuffer = Buffer.from([42]);

    storeMock = new StoreMock(this.sinon);

    transactionMock = {};

    repository = new SpentAssetLockTransactionsRepository(storeMock);
  });

  describe('#store', () => {
    it('should store outpoint', async () => {
      const repositoryInstance = await repository.store(outPointBuffer, transactionMock);
      expect(repositoryInstance).to.equal(repository);

      expect(storeMock.put).to.be.calledOnceWithExactly(
        outPointBuffer,
        Buffer.from([1]),
        transactionMock,
      );
    });
  });

  describe('#fetch', () => {
    it('should return null if outpoint is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetch(outPointBuffer, transactionMock);

      expect(result).to.be.null();

      expect(storeMock.get).to.be.calledOnceWithExactly(
        outPointBuffer,
        transactionMock,
      );
    });

    it('should return buffer containing [1]', async () => {
      storeMock.get.returns(Buffer.from([1]));

      const result = await repository.fetch(outPointBuffer, transactionMock);

      expect(result).to.be.deep.equal(Buffer.from([1]));

      expect(storeMock.get).to.be.calledOnceWithExactly(
        outPointBuffer,
        transactionMock,
      );
    });
  });
});
