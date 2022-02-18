const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');

const IdentityStoreRepository = require('../../../lib/identity/IdentityStoreRepository');
const GroveDBStoreMock = require('../../../lib/test/mock/GroveDBStoreMock');

describe('IdentityStoreRepository', () => {
  let identity;
  let repository;
  let dppMock;
  let storeMock;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock
      .identity
      .createFromBuffer
      .resolves(identity);

    storeMock = new GroveDBStoreMock(this.sinon);

    const decodeProtocolEntity = decodeProtocolEntityFactory();

    repository = new IdentityStoreRepository(storeMock, decodeProtocolEntity);
  });

  describe('#store', () => {
    it('should store identity', async () => {
      const repositoryInstance = await repository.store(identity, true);
      expect(repositoryInstance).to.equal(repository);

      expect(storeMock.put).to.be.calledOnceWithExactly(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        identity.toBuffer(),
        { useTransaction: true },
      );
    });
  });

  describe('#fetch', () => {
    it('should return null if identity is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetch(identity.getId(), true);

      expect(result).to.be.null();

      expect(storeMock.get).to.be.calledOnceWithExactly(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        { useTransaction: true },
      );
    });

    it('should return identity', async () => {
      const encodedIdentity = identity.toBuffer();

      storeMock.get.resolves(encodedIdentity);

      const result = await repository.fetch(identity.getId(), true);

      expect(result).to.be.deep.equal(identity);

      expect(storeMock.get).to.be.calledOnceWithExactly(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        { useTransaction: true },
      );
    });
  });
});
