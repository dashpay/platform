const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetDocumentsResponse,
    ResponseMetadata,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');
const documentQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/documentQueryHandlerFactory');
const InvalidQueryError = require('../../../../../lib/document/errors/InvalidQueryError');

const UnavailableAbciError = require('../../../../../lib/abci/errors/UnavailableAbciError');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const BlockExecutionContextStackMock = require('../../../../../lib/test/mock/BlockExecutionContextStackMock');
const StorageResult = require('../../../../../lib/storage/StorageResult');

describe('documentQueryHandlerFactory', () => {
  let documentQueryHandler;
  let fetchSignedDocumentsMock;
  let fetchSignedDataContractMock;
  let proveSignedDocumentsMock;
  let documents;
  let params;
  let data;
  let options;
  let createQueryResponseMock;
  let responseMock;
  let blockExecutionContextStackMock;
  let dataContractResult;

  beforeEach(function beforeEach() {
    documents = getDocumentsFixture();

    fetchSignedDocumentsMock = this.sinon.stub();
    fetchSignedDataContractMock = this.sinon.stub();
    proveSignedDocumentsMock = this.sinon.stub();
    createQueryResponseMock = this.sinon.stub();

    dataContractResult = new StorageResult(
      Buffer.alloc(40, 1),
      [new ReadOperation(40)],
    );

    responseMock = new GetDocumentsResponse();
    responseMock.setProof(new Proof());

    createQueryResponseMock.returns(responseMock);

    blockExecutionContextStackMock = new BlockExecutionContextStackMock(this.sinon);
    blockExecutionContextStackMock.getLast.returns(true);

    documentQueryHandler = documentQueryHandlerFactory(
      fetchSignedDocumentsMock,
      fetchSignedDataContractMock,
      proveSignedDocumentsMock,
      createQueryResponseMock,
      blockExecutionContextStackMock,
    );

    params = {};
    data = {
      contractId: generateRandomIdentifier(),
      type: 'documentType',
      orderBy: [{ sort: 'asc' }],
      limit: 2,
      startAt: undefined,
      startAfter: undefined,
      where: [['field', '==', 'value']],
    };
    options = {
      orderBy: data.orderBy,
      limit: data.limit,
      startAt: data.startAt,
      startAfter: data.startAfter,
      where: data.where,
    };
  });

  it('should return empty response if there is no signed state', async () => {
    blockExecutionContextStackMock.getLast.returns(null);

    responseMock = new GetDocumentsResponse();

    responseMock.setMetadata(new ResponseMetadata());

    const result = await documentQueryHandler(params, data, {});

    expect(createQueryResponseMock).to.have.not.been.called();
    expect(fetchSignedDocumentsMock).to.have.not.been.called();
    expect(fetchSignedDataContractMock).to.have.not.been.called();
    expect(proveSignedDocumentsMock).to.have.not.been.called();
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(result.value).to.deep.equal(responseMock.serializeBinary());
  });

  it('should return serialized documents', async () => {
    fetchSignedDocumentsMock.resolves(
      new StorageResult(documents),
    );

    fetchSignedDataContractMock.resolves(
      dataContractResult,
    );

    const result = await documentQueryHandler(params, data, {});

    expect(createQueryResponseMock).to.be.calledOnceWith(GetDocumentsResponse, undefined);
    expect(fetchSignedDataContractMock).to.be.calledOnceWith(data.contractId, data.type);
    expect(fetchSignedDocumentsMock).to.be.calledOnceWith(dataContractResult, data.type, options);
    expect(proveSignedDocumentsMock).to.not.be.called();
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(result.value).to.deep.equal(responseMock.serializeBinary());
  });

  it('should return proof if it was requested', async () => {
    // const proof = {
    //   rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101',
    //   'hex'),
    //   storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210',
    //   'hex'),
    // };

    const proof = Buffer.alloc(20, 255);

    fetchSignedDocumentsMock.resolves(new StorageResult(documents));
    fetchSignedDataContractMock.resolves(
      dataContractResult,
    );
    proveSignedDocumentsMock.resolves(
      new StorageResult(proof),
    );

    const result = await documentQueryHandler(params, data, { prove: true });

    expect(createQueryResponseMock).to.be.calledOnceWith(GetDocumentsResponse, true);
    expect(fetchSignedDataContractMock).to.be.calledOnceWith(data.contractId, data.type);
    expect(fetchSignedDocumentsMock).to.not.be.called();
    expect(proveSignedDocumentsMock).to.be.calledOnceWith(dataContractResult, data.type, options);

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(result.value).to.deep.equal(responseMock.serializeBinary());
  });

  it('should throw InvalidArgumentAbciError on invalid query', async () => {
    fetchSignedDataContractMock.throws(new InvalidQueryError('invalid'));

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw UnavailableAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
      expect(e.getMessage()).to.equal('Invalid query: invalid');
      expect(fetchSignedDataContractMock).to.be.calledOnceWith(data.contractId, data.type);
    }
  });

  it('should not proceed forward if createQueryResponse throws UnavailableAbciError', async () => {
    createQueryResponseMock.throws(new UnavailableAbciError('message'));

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw UnavailableAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(UnavailableAbciError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.UNAVAILABLE);
      expect(e.getMessage()).to.equal('message');
      expect(fetchSignedDocumentsMock).to.not.be.called();
    }
  });

  it('should throw error if fetchSignedDataContract throws unknown error', async () => {
    const error = new Error('Some error');

    fetchSignedDataContractMock.throws(error);

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw any error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(fetchSignedDataContractMock).to.be.calledOnceWith(data.contractId, data.type);
    }
  });
});
