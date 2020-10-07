const cbor = require('cbor');
const bs58 = require('bs58');

const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const identityIdsByPublicKeyHashesQueryHandlerFactory = require(
  '../../../../../lib/abci/handlers/query/identityIdsByPublicKeyHashesQueryHandlerFactory',
);

describe('identityIdsByPublicKeyHashesQueryHandlerFactory', () => {
  let identityIdsByPublicKeyHashesQueryHandler;
  let publicKeyIdentityIdRepositoryMock;
  let publicKeyHashes;
  let identityIds;
  let identityIdsByPublicKeyHashes;

  beforeEach(function beforeEach() {
    publicKeyIdentityIdRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    identityIdsByPublicKeyHashesQueryHandler = identityIdsByPublicKeyHashesQueryHandlerFactory(
      publicKeyIdentityIdRepositoryMock,
    );

    publicKeyHashes = [
      '784ca12495d2e61f992db9e55d1f9599b0cf1328',
      '784ca12495d2e61f992db9e55d1f9599b0cf1329',
      '784ca12495d2e61f992db9e55d1f9599b0cf1330',
    ];
    identityIds = [
      'F55Ln4ibxcZB7K9bcwCHYifCvrtQcWRWkJejPgEsz2px',
      'F55Ln4ibxcZB7K9bcwCHYifCvrtQcWRWkJejPgEsz3px',
    ];

    publicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[0])
      .resolves(identityIds[0]);

    publicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[1])
      .resolves(identityIds[1]);

    identityIdsByPublicKeyHashes = publicKeyHashes
      .map((publicKeyHash, index) => {
        if (identityIds[index]) {
          return bs58.decode(identityIds[index]);
        }

        return Buffer.alloc(0);
      });
  });

  it('should return identity id map', async () => {
    const result = await identityIdsByPublicKeyHashesQueryHandler({}, {
      publicKeyHashes,
    });

    expect(publicKeyIdentityIdRepositoryMock.fetch.callCount).to.equal(
      publicKeyHashes.length,
    );

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(cbor.encode({
      identityIds: identityIdsByPublicKeyHashes,
    }));
  });
});
