const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetIdentityResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const identityQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/identityQueryHandlerFactory');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const NotFoundAbciError = require('../../../../../lib/abci/errors/NotFoundAbciError');
const BlockExecutionContextStackMock = require('../../../../../lib/test/mock/BlockExecutionContextStackMock');
const UnimplementedAbciError = require('../../../../../lib/abci/errors/UnimplementedAbciError');

describe('identityQueryHandlerFactory', () => {
  let identityQueryHandler;
  let signedIdentityRepository;
  let identity;
  let params;
  let data;
  let createQueryResponseMock;
  let responseMock;
  let blockExecutionContextMock;
  let blockExecutionContextStackMock;

  beforeEach(function beforeEach() {
    signedIdentityRepository = {
      fetch: this.sinon.stub(),
    };

    createQueryResponseMock = this.sinon.stub();

    responseMock = new GetIdentityResponse();
    responseMock.setProof(new Proof());

    createQueryResponseMock.returns(responseMock);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextStackMock = new BlockExecutionContextStackMock(this.sinon);
    blockExecutionContextStackMock.getLast.returns(true);

    identityQueryHandler = identityQueryHandlerFactory(
      signedIdentityRepository,
      createQueryResponseMock,
      blockExecutionContextMock,
      blockExecutionContextStackMock,
    );

    identity = getIdentityFixture();

    params = {};
    data = {
      id: identity.getId(),
    };
  });

  it('should throw NotFoundAbciError if there is no signed state', async () => {
    blockExecutionContextStackMock.getLast.returns(null);

    try {
      await identityQueryHandler(params, data, {});

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NotFoundAbciError);
    }
  });

  it('should return serialized identity', async () => {
    signedIdentityRepository.fetch.resolves(identity);

    const result = await identityQueryHandler(params, data, {});

    expect(signedIdentityRepository.fetch).to.be.calledOnceWith(data.id);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());
  });

  it('should throw NotFoundAbciError if identity not found', async () => {
    try {
      await identityQueryHandler(params, data, {});

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(NotFoundAbciError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.NOT_FOUND);
      expect(e.message).to.equal('Identity not found');
      expect(signedIdentityRepository.fetch).to.be.calledOnceWith(data.id);
    }
  });

  it('should throw UnimplementedAbciError if proof requested', async () => {
    // const proof = {
    //   rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101',
    //     'hex'),
    //   storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210',
    //     'hex'),
    // };

    try {
      await identityQueryHandler(params, data, { prove: true });

      expect.fail('should throw UnimplementedAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(UnimplementedAbciError);
    }
  });
});
