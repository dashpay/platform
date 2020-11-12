const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');
const cbor = require('cbor');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const identityQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/identityQueryHandlerFactory');

const NotFoundAbciError = require('../../../../../lib/abci/errors/NotFoundAbciError');
const AbciError = require('../../../../../lib/abci/errors/AbciError');

describe('identityQueryHandlerFactory', () => {
  let identityQueryHandler;
  let identityRepositoryMock;
  let identity;
  let params;
  let data;
  let rootTreeMock;
  let identitiesStoreRootTreeLeafMock;

  beforeEach(function beforeEach() {
    identityRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    rootTreeMock = {
      getFullProof: this.sinon.stub(),
    };

    identitiesStoreRootTreeLeafMock = this.sinon.stub();

    identityQueryHandler = identityQueryHandlerFactory(
      identityRepositoryMock,
      rootTreeMock,
      identitiesStoreRootTreeLeafMock,
    );

    identity = getIdentityFixture();

    params = {};
    data = {
      id: identity.getId(),
    };
  });

  it('should return serialized identity', async () => {
    identityRepositoryMock.fetch.resolves(identity);

    const result = await identityQueryHandler(params, data, {});

    expect(identityRepositoryMock.fetch).to.be.calledOnceWith(data.id);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(cbor.encode({
      data: identity.toBuffer(),
    }));
  });

  it('should throw NotFoundAbciError if identity not found', async () => {
    try {
      await identityQueryHandler(params, data, {});

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(NotFoundAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.NOT_FOUND);
      expect(e.message).to.equal('Identity not found');
      expect(identityRepositoryMock.fetch).to.be.calledOnceWith(data.id);
    }
  });

  it('should return serialized identity with proof', async () => {
    const proof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    identityRepositoryMock.fetch.resolves(identity);
    rootTreeMock.getFullProof.returns(proof);

    const result = await identityQueryHandler(params, data, { prove: 'true' });
    expect(identityRepositoryMock.fetch).to.be.calledOnceWith(data.id);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    const value = {
      data: identity.toBuffer(),
      proof,
    };

    expect(result.value).to.deep.equal(cbor.encode(value));
    expect(rootTreeMock.getFullProof).to.be.calledOnceWith(
      identitiesStoreRootTreeLeafMock,
      [identity.getId()],
    );
  });
});
