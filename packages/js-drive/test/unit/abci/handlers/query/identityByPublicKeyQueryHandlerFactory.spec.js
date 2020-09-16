const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const identityByPublicKeyHashQueryHandlerFactory = require(
  '../../../../../lib/abci/handlers/query/identityByPublicKeyHashQueryHandlerFactory',
);

const NotFoundAbciError = require('../../../../../lib/abci/errors/NotFoundAbciError');
const AbciError = require('../../../../../lib/abci/errors/AbciError');

describe('identityByPublicKeyHashQueryHandlerFactory', () => {
  let identityRepositoryMock;
  let publicKeyIdentityIdRepositoryMock;
  let identityByPublicKeyHashQueryHandler;
  let publicKeyHash;
  let identity;
  let identityId;

  beforeEach(function beforeEach() {
    identityRepositoryMock = {
      fetch: this.sinon.stub(),
    };
    publicKeyIdentityIdRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    identityByPublicKeyHashQueryHandler = identityByPublicKeyHashQueryHandlerFactory(
      publicKeyIdentityIdRepositoryMock,
      identityRepositoryMock,
    );

    publicKeyHash = 'publicKeyHash';
    identityId = 'identityId';

    identity = getIdentityFixture();
  });

  it('should return serialized identity', async () => {
    identityRepositoryMock.fetch.resolves(identity);
    publicKeyIdentityIdRepositoryMock.fetch.resolves(identityId);

    const result = await identityByPublicKeyHashQueryHandler({ publicKeyHash });

    expect(identityRepositoryMock.fetch).to.be.calledOnceWith(identityId);
    expect(publicKeyIdentityIdRepositoryMock.fetch).to.be.calledOnceWith(publicKeyHash);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(identity.serialize());
  });

  it('should throw NotFoundAbciError if identityId not found', async () => {
    try {
      await identityByPublicKeyHashQueryHandler({ publicKeyHash });

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(NotFoundAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.NOT_FOUND);
      expect(e.message).to.equal('Identity not found');
      expect(publicKeyIdentityIdRepositoryMock.fetch).to.be.calledOnceWith(publicKeyHash);
      expect(identityRepositoryMock.fetch).to.be.not.called();
    }
  });

  it('should throw NotFoundAbciError if identity not found', async () => {
    publicKeyIdentityIdRepositoryMock.fetch.resolves(identityId);

    try {
      await identityByPublicKeyHashQueryHandler({ publicKeyHash });

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(NotFoundAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.NOT_FOUND);
      expect(e.message).to.equal('Identity not found');
      expect(publicKeyIdentityIdRepositoryMock.fetch).to.be.calledOnceWith(publicKeyHash);
      expect(identityRepositoryMock.fetch).to.be.calledOnceWith(identityId);
    }
  });
});
