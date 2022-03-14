const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const cbor = require('cbor');

const PublicKeyToIdentityIdStoreRepository = require('../../../lib/identity/PublicKeyToIdentityIdStoreRepository');
const GroveDBStoreMock = require('../../../lib/test/mock/GroveDBStoreMock');

describe('PublicKeyToIdentityIdStoreRepository', () => {
  let identity;
  let repository;
  let storeMock;
  let publicKey;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();
    publicKey = new IdentityPublicKey({
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('A3NCyQmImEGgr7j2kR+rTumHLORpCGYC6XeCqVODZgSm', 'base64'),
    });
    storeMock = new GroveDBStoreMock(this.sinon);

    repository = new PublicKeyToIdentityIdStoreRepository(storeMock);
  });

  describe('#store', () => {
    it('should not store identity id if it is already stored', async () => {
      storeMock.get.returns(
        cbor.encode([identity.getId()]),
      );

      await repository.store(
        publicKey.hash(),
        identity.getId(),
      );

      expect(storeMock.put).to.have.not.been.called();
    });

    it('should store public key to identity id map', async () => {
      const repositoryInstance = await repository.store(
        publicKey.hash(),
        identity.getId(),
        true,
      );
      expect(repositoryInstance).to.equal(repository);

      expect(storeMock.put).to.be.calledOnceWithExactly(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKey.hash(),
        cbor.encode([identity.getId()]),
        { useTransaction: true },
      );
    });
  });

  describe('#fetchBuffer', () => {
    it('should return null if publicKeyHash is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetchBuffer(publicKey.hash(), true);

      expect(result).to.be.null();

      expect(storeMock.get).to.be.calledOnceWithExactly(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKey.hash(),
        { useTransaction: true },
      );
    });

    it('should return buffer', async () => {
      storeMock.get.returns(cbor.encode([]));

      const result = await repository.fetchBuffer(publicKey.hash(), true);

      expect(result).to.be.deep.equal(cbor.encode([]));

      expect(storeMock.get).to.be.calledOnceWithExactly(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKey.hash(),
        { useTransaction: true },
      );
    });
  });

  describe('#fetch', () => {
    it('should return empty array if publicKeyHash is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetch(publicKey.hash(), true);

      expect(result).to.deep.equal([]);

      expect(storeMock.get).to.be.calledOnceWithExactly(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKey.hash(),
        { useTransaction: true },
      );
    });

    it('should return array of Identity ids', async () => {
      storeMock.get.returns(cbor.encode([identity.getId().toBuffer()]));

      const result = await repository.fetch(publicKey.hash(), true);

      expect(result).to.have.deep.members([
        identity.getId(),
      ]);

      expect(storeMock.get).to.be.calledOnceWithExactly(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKey.hash(),
        { useTransaction: true },
      );
    });
  });
});
