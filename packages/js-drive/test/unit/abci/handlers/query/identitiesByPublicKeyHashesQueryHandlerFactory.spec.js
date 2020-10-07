const cbor = require('cbor');

const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const identitiesByPublicKeyHashesQueryHandlerFactory = require(
  '../../../../../lib/abci/handlers/query/identitiesByPublicKeyHashesQueryHandlerFactory',
);

describe('identitiesByPublicKeyHashesQueryHandlerFactory', () => {
  let identitiesByPublicKeyHashesQueryHandler;
  let publicKeyIdentityIdRepositoryMock;
  let identityRepositoryMock;
  let publicKeyHashes;
  let identityIds;
  let identities;
  let identitiesByPublicKeyHashes;

  beforeEach(function beforeEach() {
    publicKeyIdentityIdRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    identityRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    identitiesByPublicKeyHashesQueryHandler = identitiesByPublicKeyHashesQueryHandlerFactory(
      publicKeyIdentityIdRepositoryMock,
      identityRepositoryMock,
    );

    publicKeyHashes = [
      '784ca12495d2e61f992db9e55d1f9599b0cf1328',
      '784ca12495d2e61f992db9e55d1f9599b0cf1329',
      '784ca12495d2e61f992db9e55d1f9599b0cf1330',
    ];
    identities = [
      getIdentityFixture(),
      getIdentityFixture(),
    ];
    identityIds = identities.map((identity) => identity.getId());

    publicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[0])
      .resolves(identityIds[0]);

    publicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[1])
      .resolves(identityIds[1]);

    identityRepositoryMock.fetch
      .withArgs(identityIds[0])
      .resolves(identities[0]);

    identityRepositoryMock.fetch
      .withArgs(identityIds[1])
      .resolves(identities[1]);

    identitiesByPublicKeyHashes = publicKeyHashes.map((publicKeyHash, index) => {
      const identity = identities[index];

      if (!identity) {
        return Buffer.alloc(0);
      }

      return identity.serialize();
    });
  });

  it('should return identity id map', async () => {
    const result = await identitiesByPublicKeyHashesQueryHandler({}, {
      publicKeyHashes,
    });

    expect(publicKeyIdentityIdRepositoryMock.fetch.callCount).to.equal(
      publicKeyHashes.length,
    );
    expect(identityRepositoryMock.fetch.callCount).to.equal(
      identityIds.length,
    );

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(cbor.encode({
      identities: identitiesByPublicKeyHashes,
    }));
  });
});
