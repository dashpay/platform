const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const cbor = require('cbor');
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
    it('should not store identity id if it is already stored', async () => {
      storeMock.get.returns(
        cbor.encode([identity.getId()]),
      );

      await repository.store(
        publicKey.hash(),
        identity.getId(),
        transactionMock,
      );

      expect(storeMock.put).to.have.not.been.called();
    });

    it('should store public key to identity id map', async () => {
      const repositoryInstance = await repository.store(
        publicKey.hash(),
        identity.getId(),
        transactionMock,
      );
      expect(repositoryInstance).to.equal(repository);

      expect(storeMock.put).to.be.calledOnceWithExactly(
        publicKey.hash(),
        cbor.encode([identity.getId()]),
        transactionMock,
      );
    });
  });

  describe('#fetchBuffer', () => {
    it('should return null if publicKeyHash is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetchBuffer(publicKey.hash(), transactionMock);

      expect(result).to.be.null();

      expect(storeMock.get).to.be.calledOnceWithExactly(
        publicKey.hash(),
        transactionMock,
      );
    });

    it('should return buffer', async () => {
      storeMock.get.returns(cbor.encode([]));

      const result = await repository.fetchBuffer(publicKey.hash(), transactionMock);

      expect(result).to.be.deep.equal(cbor.encode([]));

      expect(storeMock.get).to.be.calledOnceWithExactly(
        publicKey.hash(),
        transactionMock,
      );
    });
  });

  describe('#fetch', () => {
    it('should return empty array if publicKeyHash is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetch(publicKey.hash(), transactionMock);

      expect(result).to.deep.equal([]);

      expect(storeMock.get).to.be.calledOnceWithExactly(
        publicKey.hash(),
        transactionMock,
      );
    });

    it('should return array of Identity ids', async () => {
      storeMock.get.returns(cbor.encode([identity.getId().toBuffer()]));

      const result = await repository.fetch(publicKey.hash(), transactionMock);

      expect(result).to.have.deep.members([
        identity.getId(),
      ]);

      expect(storeMock.get).to.be.calledOnceWithExactly(
        publicKey.hash(),
        transactionMock,
      );
    });
  });
});
