const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const StoreMock = require('../../../lib/test/mock/StoreMock');

const PublicKeyToIdentityIdStoreRepository = require('../../../lib/identity/PublicKeyToIdentityIdStoreRepository');

describe('PublicKeyToIdentityIdStoreRepository', () => {
  let identity;
  let repository;
  let storeMock;
  let transactionMock;
  let publicKey;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();
    publicKey = new IdentityPublicKey({
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('A3NCyQmImEGgr7j2kR+rTumHLORpCGYC6XeCqVODZgSm', 'base64'),
    });
    storeMock = new StoreMock(this.sinon);

    transactionMock = {};

    repository = new PublicKeyToIdentityIdStoreRepository(storeMock);
  });

  describe('#store', () => {
    it('should store public key to identity id map', async () => {
      const repositoryInstance = await repository.store(
        publicKey.hash(),
        identity.getId(),
        transactionMock,
      );
      expect(repositoryInstance).to.equal(repository);

      expect(storeMock.put).to.be.calledOnceWithExactly(
        publicKey.hash(),
        identity.getId(),
        transactionMock,
      );
    });
  });

  describe('#fetch', () => {
    it('should return null if publicKeyHash is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetch(publicKey.hash(), transactionMock);

      expect(result).to.be.null();

      expect(storeMock.get).to.be.calledOnceWithExactly(
        publicKey.hash(),
        transactionMock,
      );
    });

    it('should return identity Id', async () => {
      storeMock.get.returns(identity.getId().toBuffer());

      const result = await repository.fetch(publicKey.hash(), transactionMock);

      expect(result).to.be.deep.equal(identity.getId());

      expect(storeMock.get).to.be.calledOnceWithExactly(
        publicKey.hash(),
        transactionMock,
      );
    });
  });
});
