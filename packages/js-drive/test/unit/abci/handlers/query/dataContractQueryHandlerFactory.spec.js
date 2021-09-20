const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetDataContractResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const dataContractQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/dataContractQueryHandlerFactory');

const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const NotFoundAbciError = require('../../../../../lib/abci/errors/NotFoundAbciError');

describe('dataContractQueryHandlerFactory', () => {
  let dataContractQueryHandler;
  let previousDataContractRepositoryMock;
  let dataContract;
  let params;
  let data;
  let previousRootTreeMock;
  let previousDataContractsStoreRootTreeLeafMock;
  let createQueryResponseMock;
  let responseMock;
  let blockExecutionContextMock;
  let previousBlockExecutionContextMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();

    previousDataContractRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    previousRootTreeMock = {
      getFullProofForOneLeaf: this.sinon.stub(),
    };

    previousDataContractsStoreRootTreeLeafMock = this.sinon.stub();
    createQueryResponseMock = this.sinon.stub();

    responseMock = new GetDataContractResponse();
    responseMock.setProof(new Proof());

    createQueryResponseMock.returns(responseMock);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    previousBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    dataContractQueryHandler = dataContractQueryHandlerFactory(
      previousDataContractRepositoryMock,
      previousRootTreeMock,
      previousDataContractsStoreRootTreeLeafMock,
      createQueryResponseMock,
      blockExecutionContextMock,
      previousBlockExecutionContextMock,
    );

    params = { };
    data = {
      id: dataContract.getId(),
    };
  });

  it('should throw NotFoundAbciError if blockExecutionContext is empty', async () => {
    blockExecutionContextMock.isEmpty.returns(true);

    try {
      await dataContractQueryHandler(params, data, {});

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NotFoundAbciError);
    }
  });

  it('should throw NotFoundAbciError if previousBlockExecutionContext is empty', async () => {
    previousBlockExecutionContextMock.isEmpty.returns(true);

    try {
      await dataContractQueryHandler(params, data, {});

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NotFoundAbciError);
    }
  });

  it('should return data contract', async () => {
    previousDataContractRepositoryMock.fetch.resolves(dataContract);

    const result = await dataContractQueryHandler(params, data, {});

    expect(previousDataContractRepositoryMock.fetch).to.be.calledOnceWith(data.id);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());
    expect(previousRootTreeMock.getFullProofForOneLeaf).not.to.be.called();
  });

  it('should return proof', async () => {
    const proof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    previousDataContractRepositoryMock.fetch.resolves(dataContract);
    previousRootTreeMock.getFullProofForOneLeaf.returns(proof);

    const result = await dataContractQueryHandler(params, data, { prove: true });

    expect(previousDataContractRepositoryMock.fetch).to.be.calledOnceWith(data.id);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());
    expect(previousRootTreeMock.getFullProofForOneLeaf).to.be.calledOnce();
    expect(previousRootTreeMock.getFullProofForOneLeaf.getCall(0).args).to.deep.equal([
      previousDataContractsStoreRootTreeLeafMock,
      [dataContract.getId()],
    ]);
  });

  it('should throw NotFoundAbciError if data contract not found', async () => {
    try {
      await dataContractQueryHandler(params, data, {});

      expect.fail('should throw NotFoundAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(NotFoundAbciError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.NOT_FOUND);
      expect(e.message).to.equal('Data Contract not found');
      expect(previousDataContractRepositoryMock.fetch).to.be.calledOnceWith(data.id);
    }
  });
});
