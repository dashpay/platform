const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const identityQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/identityQueryHandlerFactory');

const NotFoundAbciError = require('../../../../../lib/abci/errors/NotFoundAbciError');
const AbciError = require('../../../../../lib/abci/errors/AbciError');

describe('identityQueryHandlerFactory', () => {
  let identityQueryHandler;
  let identityRepositoryMock;
  let identity;

  beforeEach(function beforeEach() {
    identityRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    identityQueryHandler = identityQueryHandlerFactory(
      identityRepositoryMock,
    );

    identity = getIdentityFixture();
  });

  it('should return serialized identity', async () => {
    const id = 'id';

    identityRepositoryMock.fetch.resolves(identity);

    const result = await identityQueryHandler({ id });

    expect(identityRepositoryMock.fetch).to.be.calledOnceWith(id);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(identity.serialize());
  });

  it('should throw NotFoundAbciError if identity not found', async () => {
    const id = 'id';

    try {
      await identityQueryHandler({ id });

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(NotFoundAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.NOT_FOUND);
      expect(e.message).to.equal('Identity not found');
      expect(identityRepositoryMock.fetch).to.be.calledOnceWith(id);
    }
  });
});
