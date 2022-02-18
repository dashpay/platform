const GroveDBStoreMock = require('../../../lib/test/mock/GroveDBStoreMock');

const SpentAssetLockTransactionsRepository = require('../../../lib/identity/SpentAssetLockTransactionsRepository');

describe('SpentAssetLockTransactionsRepository', () => {
  let outPointBuffer;
  let repository;
  let storeMock;

  beforeEach(function beforeEach() {
    outPointBuffer = Buffer.from([42]);

    storeMock = new GroveDBStoreMock(this.sinon);

    repository = new SpentAssetLockTransactionsRepository(storeMock);
  });

  describe('#store', () => {
    it('should store outpoint', async () => {
      const repositoryInstance = await repository.store(outPointBuffer, true);
      expect(repositoryInstance).to.equal(repository);

      expect(storeMock.put).to.be.calledOnceWithExactly(
        SpentAssetLockTransactionsRepository.TREE_PATH,
        outPointBuffer,
        Buffer.from([0]),
        { useTransaction: true },
      );
    });
  });

  describe('#fetch', () => {
    it('should return null if outpoint is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetch(outPointBuffer, true);

      expect(result).to.be.null();

      expect(storeMock.get).to.be.calledOnceWithExactly(
        SpentAssetLockTransactionsRepository.TREE_PATH,
        outPointBuffer,
        { useTransaction: true },
      );
    });

    it('should return buffer containing [1]', async () => {
      storeMock.get.returns(Buffer.from([1]));

      const result = await repository.fetch(outPointBuffer, true);

      expect(result).to.be.deep.equal(Buffer.from([1]));

      expect(storeMock.get).to.be.calledOnceWithExactly(
        SpentAssetLockTransactionsRepository.TREE_PATH,
        outPointBuffer,
        { useTransaction: true },
      );
    });
  });
});
