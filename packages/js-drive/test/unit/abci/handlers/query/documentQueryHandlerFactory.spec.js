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
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const documentQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/documentQueryHandlerFactory');
const InvalidQueryError = require('../../../../../lib/document/errors/InvalidQueryError');
const ValidationError = require('../../../../../lib/document/query/errors/ValidationError');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const AbciError = require('../../../../../lib/abci/errors/AbciError');
const UnavailableAbciError = require('../../../../../lib/abci/errors/UnavailableAbciError');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');

describe('documentQueryHandlerFactory', () => {
  let documentQueryHandler;
  let fetchPreviousDocumentsMock;
  let documents;
  let params;
  let data;
  let options;
  let previousRootTreeMock;
  let previousDocumentsStoreRootTreeLeafMock;
  let containerMock;
  let previousBlockExecutionTransactionsMock;
  let transactionMock;
  let createQueryResponseMock;
  let responseMock;
  let blockExecutionContextMock;
  let previousBlockExecutionContextMock;

  beforeEach(function beforeEach() {
    documents = getDocumentsFixture();

    fetchPreviousDocumentsMock = this.sinon.stub();

    previousRootTreeMock = {
      getFullProof: this.sinon.stub(),
    };

    previousDocumentsStoreRootTreeLeafMock = this.sinon.stub();

    transactionMock = {
      isStarted: this.sinon.stub().returns(true),
    };

    previousBlockExecutionTransactionsMock = {
      getTransaction: this.sinon.stub().returns(transactionMock),
    };

    containerMock = {
      has: this.sinon.stub().returns(true),
      resolve: this.sinon.stub().returns(previousBlockExecutionTransactionsMock),
    };

    createQueryResponseMock = this.sinon.stub();

    responseMock = new GetDocumentsResponse();
    responseMock.setProof(new Proof());

    createQueryResponseMock.returns(responseMock);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    previousBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    documentQueryHandler = documentQueryHandlerFactory(
      fetchPreviousDocumentsMock,
      previousRootTreeMock,
      previousDocumentsStoreRootTreeLeafMock,
      containerMock,
      createQueryResponseMock,
      blockExecutionContextMock,
      previousBlockExecutionContextMock,
    );

    params = {};
    data = {
      contractId: generateRandomIdentifier(),
      type: 'documentType',
      orderBy: [{ sort: 'asc' }],
      limit: 2,
      startAt: 0,
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

  it('should return empty response if blockExecutionContext is empty', async () => {
    previousBlockExecutionContextMock.isEmpty.returns(true);

    responseMock = new GetDocumentsResponse();

    const result = await documentQueryHandler(params, data, {});

    expect(fetchPreviousDocumentsMock).to.have.not.been.called();
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(result.value).to.deep.equal(responseMock.serializeBinary());
    expect(previousRootTreeMock.getFullProof).to.have.not.been.called();
  });

  it('should return empty response if previousBlockExecutionContext is empty', async () => {
    previousBlockExecutionContextMock.isEmpty.returns(true);

    responseMock = new GetDocumentsResponse();

    const result = await documentQueryHandler(params, data, {});

    expect(fetchPreviousDocumentsMock).to.have.not.been.called();
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(result.value).to.deep.equal(responseMock.serializeBinary());
    expect(previousRootTreeMock.getFullProof).to.have.not.been.called();
  });

  it('should return serialized documents', async () => {
    fetchPreviousDocumentsMock.resolves(documents);

    const result = await documentQueryHandler(params, data, {});

    expect(fetchPreviousDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(result.value).to.deep.equal(responseMock.serializeBinary());
    expect(previousRootTreeMock.getFullProof).to.be.not.called();
    expect(containerMock.has).to.be.calledOnceWithExactly('previousBlockExecutionStoreTransactions');
  });

  it('should return serialized documents with proof', async () => {
    const proof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    fetchPreviousDocumentsMock.resolves(documents);
    previousRootTreeMock.getFullProof.returns(proof);

    const result = await documentQueryHandler(params, data, { prove: 'true' });

    expect(fetchPreviousDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    const documentIds = documents.map((document) => document.getId());

    expect(result.value).to.deep.equal(responseMock.serializeBinary());
    expect(previousRootTreeMock.getFullProof).to.be.calledOnce();
    expect(previousRootTreeMock.getFullProof.getCall(0).args).to.deep.equal([
      previousDocumentsStoreRootTreeLeafMock,
      documentIds,
    ]);
    expect(containerMock.has).to.be.calledOnceWithExactly('previousBlockExecutionStoreTransactions');
  });

  it('should throw UnavailableAbciError if previousBlockExecutionStoreTransactions is not present', async () => {
    containerMock.has.returns(false);

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw UnavailableAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(UnavailableAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.UNAVAILABLE);
      expect(fetchPreviousDocumentsMock).to.not.be.called();
      expect(containerMock.has).to.be.calledOnceWithExactly('previousBlockExecutionStoreTransactions');
    }
  });

  it('should throw UnavailableAbciError if transaction is not started', async () => {
    transactionMock.isStarted.returns(false);

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw UnavailableAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(UnavailableAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.UNAVAILABLE);
      expect(fetchPreviousDocumentsMock).to.not.be.called();
      expect(containerMock.resolve).to.be.calledOnceWithExactly('previousBlockExecutionStoreTransactions');
      expect(previousBlockExecutionTransactionsMock.getTransaction).to.be.calledOnceWithExactly('dataContracts');
      expect(transactionMock.isStarted).to.be.calledOnce();
    }
  });

  it('should not proceed forward if createQueryResponse throws UnavailableAbciError', async () => {
    createQueryResponseMock.throws(new UnavailableAbciError());

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw UnavailableAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(UnavailableAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.UNAVAILABLE);
      expect(fetchPreviousDocumentsMock).to.not.be.called();
      expect(containerMock.resolve).to.be.calledOnceWithExactly('previousBlockExecutionStoreTransactions');
      expect(previousBlockExecutionTransactionsMock.getTransaction).to.be.calledOnceWithExactly('dataContracts');
      expect(transactionMock.isStarted).to.be.calledOnce();
    }
  });

  it('should throw InvalidArgumentAbciError on invalid query', async () => {
    const error = new ValidationError('Some error');
    const queryError = new InvalidQueryError([error]);

    fetchPreviousDocumentsMock.throws(queryError);

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);
      expect(e.getData()).to.deep.equal({ errors: [error] });
      expect(fetchPreviousDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
      expect(containerMock.has).to.be.calledOnceWithExactly('previousBlockExecutionStoreTransactions');
    }
  });

  it('should throw error if fetchDocuments throws unknown error', async () => {
    const error = new Error('Some error');

    fetchPreviousDocumentsMock.throws(error);

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw any error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(fetchPreviousDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
      expect(containerMock.has).to.be.calledOnceWithExactly('previousBlockExecutionStoreTransactions');
    }
  });
});
