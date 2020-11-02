const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const StoreMock = require('../../../lib/test/mock/StoreMock');

const IdentityStoreRepository = require('../../../lib/identity/IdentityStoreRepository');

describe('IdentityStoreRepository', () => {
  let identity;
  let repository;
  let dppMock;
  let storeMock;
  let transactionMock;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock
      .identity
      .createFromBuffer
      .resolves(identity);

    storeMock = new StoreMock(this.sinon);

    transactionMock = {};

    repository = new IdentityStoreRepository(storeMock, dppMock);
  });

  describe('#store', () => {
    it('should store identity', async () => {
      const repositoryInstance = await repository.store(identity, transactionMock);
      expect(repositoryInstance).to.equal(repository);

      expect(storeMock.put).to.be.calledOnceWithExactly(
        identity.getId(),
        identity.toBuffer(),
        transactionMock,
      );
    });
  });

  describe('#fetch', () => {
    it('should return null if identity is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetch(identity.getId(), transactionMock);

      expect(result).to.be.null();

      expect(storeMock.get).to.be.calledOnceWithExactly(
        identity.getId(),
        transactionMock,
      );
    });

    it('should return identity', async () => {
      const encodedIdentitiy = identity.toBuffer();

      storeMock.get.returns(encodedIdentitiy);

      const result = await repository.fetch(identity.getId(), transactionMock);

      expect(result).to.be.deep.equal(identity);

      expect(storeMock.get).to.be.calledOnceWithExactly(
        identity.getId(),
        transactionMock,
      );
    });
  });
});
