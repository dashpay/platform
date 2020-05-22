const bs58 = require('bs58');

const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const identityIdByFirstPublicKeyQueryHandlerFactory = require(
  '../../../../../lib/abci/handlers/query/identityIdByFirstPublicKeyQueryHandlerFactory',
);

const NotFoundAbciError = require('../../../../../lib/abci/errors/NotFoundAbciError');
const AbciError = require('../../../../../lib/abci/errors/AbciError');

describe('identityIdByFirstPublicKeyQueryHandlerFactory', () => {
  let identityIdByFirstPublicKeyQueryHandler;
  let publicKeyIdentityIdRepositoryMock;
  let publicKeyHash;
  let identityId;

  beforeEach(function beforeEach() {
    publicKeyIdentityIdRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    identityIdByFirstPublicKeyQueryHandler = identityIdByFirstPublicKeyQueryHandlerFactory(
      publicKeyIdentityIdRepositoryMock,
    );

    publicKeyHash = '784ca12495d2e61f992db9e55d1f9599b0cf1328';
    identityId = 'F55Ln4ibxcZB7K9bcwCHYifCvrtQcWRWkJejPgEsz2px';
  });

  it('should return identity id', async () => {
    publicKeyIdentityIdRepositoryMock.fetch.resolves(identityId);

    const result = await identityIdByFirstPublicKeyQueryHandler({ publicKeyHash });

    expect(publicKeyIdentityIdRepositoryMock.fetch).to.be.calledOnceWith(publicKeyHash);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(bs58.encode(result.value)).to.equal(identityId);
  });

  it('should throw NotFoundAbciError if identityId not found', async () => {
    try {
      await identityIdByFirstPublicKeyQueryHandler({ publicKeyHash });

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(NotFoundAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.NOT_FOUND);
      expect(e.message).to.equal('Identity not found');
      expect(publicKeyIdentityIdRepositoryMock.fetch).to.be.calledOnceWith(publicKeyHash);
    }
  });
});
