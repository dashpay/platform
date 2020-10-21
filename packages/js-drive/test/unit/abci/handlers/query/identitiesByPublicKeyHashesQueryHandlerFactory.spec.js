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
      Buffer.from('784ca12495d2e61f992db9e55d1f9599b0cf1328', 'hex'),
      Buffer.from('784ca12495d2e61f992db9e55d1f9599b0cf1329', 'hex'),
      Buffer.from('784ca12495d2e61f992db9e55d1f9599b0cf1330', 'hex'),
    ];

    identities = [
      getIdentityFixture(),
      getIdentityFixture(),
    ];

    publicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[0])
      .resolves(identities[0].getId());

    publicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[1])
      .resolves(identities[1].getId());

    identityRepositoryMock.fetch
      .withArgs(identities[0].getId())
      .resolves(identities[0]);

    identityRepositoryMock.fetch
      .withArgs(identities[0].getId())
      .resolves(identities[1]);

    identitiesByPublicKeyHashes = [
      identities[0].toBuffer(),
      identities[1].toBuffer(),
      Buffer.alloc(0),
    ];
  });

  it('should return identity id map', async () => {
    const params = {};
    const data = { publicKeyHashes };

    const result = await identitiesByPublicKeyHashesQueryHandler(params, data);

    expect(publicKeyIdentityIdRepositoryMock.fetch.callCount).to.equal(
      publicKeyHashes.length,
    );

    expect(publicKeyIdentityIdRepositoryMock.fetch.getCall(0).args).to.deep.equal([
      publicKeyHashes[0],
    ]);

    expect(publicKeyIdentityIdRepositoryMock.fetch.getCall(1).args).to.deep.equal([
      publicKeyHashes[1],
    ]);

    expect(publicKeyIdentityIdRepositoryMock.fetch.getCall(2).args).to.deep.equal([
      publicKeyHashes[2],
    ]);

    expect(identityRepositoryMock.fetch.callCount).to.equal(
      identities.length,
    );

    expect(identityRepositoryMock.fetch.getCall(0).args).to.deep.equal([
      identities[0].getId(),
    ]);

    expect(identityRepositoryMock.fetch.getCall(1).args).to.deep.equal([
      identities[1].getId(),
    ]);

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(await cbor.encodeAsync(identitiesByPublicKeyHashes));
  });
});
