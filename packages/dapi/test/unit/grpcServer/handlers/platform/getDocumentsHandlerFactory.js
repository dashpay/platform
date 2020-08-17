const cbor = require('cbor');

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetDocumentsResponse,
  },
} = require('@dashevo/dapi-grpc');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getDocumentsHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getDocumentsHandlerFactory',
);

const AbciResponseError = require('../../../../../lib/errors/AbciResponseError');

describe('getDocumentsHandlerFactory', () => {
  let call;
  let getDocumentsHandler;
  let driveStateRepositoryMock;
  let request;
  let documentsFixture;
  let dataContractId;
  let documentType;
  let where;
  let orderBy;
  let limit;
  let startAfter;
  let startAt;
  let handleAbciResponseErrorMock;
  let documentsSerialized;

  beforeEach(function beforeEach() {
    dataContractId = 'contractId';
    documentType = 'document';
    where = [['name', '==', 'John']];
    orderBy = [{ order: 'asc' }];
    limit = 20;
    startAfter = 1;
    startAt = null;

    request = {
      getDataContractId: this.sinon.stub().returns(dataContractId),
      getDocumentType: this.sinon.stub().returns(documentType),
      getWhere: this.sinon.stub().returns(new Uint8Array(cbor.encode(where))),
      getOrderBy: this.sinon.stub().returns(new Uint8Array(cbor.encode(orderBy))),
      getLimit: this.sinon.stub().returns(limit),
      getStartAfter: this.sinon.stub().returns(startAfter),
      getStartAt: this.sinon.stub().returns(startAt),
    };

    call = new GrpcCallMock(this.sinon, request);

    const [document] = getDocumentsFixture();

    documentsFixture = [document];

    documentsSerialized = documentsFixture.map(documentItem => documentItem.serialize());

    driveStateRepositoryMock = {
      fetchDocuments: this.sinon.stub().resolves(documentsSerialized),
    };

    handleAbciResponseErrorMock = this.sinon.stub();

    getDocumentsHandler = getDocumentsHandlerFactory(
      driveStateRepositoryMock,
      handleAbciResponseErrorMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getDocumentsHandler(call);

    expect(result).to.be.an.instanceOf(GetDocumentsResponse);

    const documentsBinary = result.getDocumentsList();
    expect(documentsBinary).to.be.an('array');
    expect(documentsBinary).to.have.lengthOf(documentsFixture.length);

    expect(driveStateRepositoryMock.fetchDocuments).to.be.calledOnceWith(
      dataContractId,
      documentType,
      {
        where,
        orderBy,
        limit,
        startAfter,
        startAt: undefined,
      },
    );

    expect(documentsBinary[0]).to.deep.equal(documentsSerialized[0]);
    expect(handleAbciResponseErrorMock).to.be.not.called();
  });

  it('should throw InvalidArgumentGrpcError if dataContractId is not specified', async () => {
    dataContractId = null;
    request.getDataContractId.returns(dataContractId);

    try {
      await getDocumentsHandler(call);

      expect.fail('should throw InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('dataContractId is not specified');
      expect(driveStateRepositoryMock.fetchDocuments).to.be.not.called();
      expect(handleAbciResponseErrorMock).to.be.not.called();
    }
  });

  it('should throw InvalidArgumentGrpcError if documentType is not specified', async () => {
    documentType = null;
    request.getDocumentType.returns(documentType);

    try {
      await getDocumentsHandler(call);

      expect.fail('should throw InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('documentType is not specified');
      expect(driveStateRepositoryMock.fetchDocuments).to.be.not.called();
      expect(handleAbciResponseErrorMock).to.be.not.called();
    }
  });

  it('should throw InvalidArgumentGrpcError if fetchDocuments throws AbciResponseError', async () => {
    const code = 2;
    const message = 'Some error';
    const data = 42;
    const abciResponseError = new AbciResponseError(code, { message, data });
    const handleError = new InvalidArgumentGrpcError('Another error');

    handleAbciResponseErrorMock.throws(handleError);

    driveStateRepositoryMock.fetchDocuments.throws(abciResponseError);

    try {
      await getDocumentsHandler(call);

      expect.fail('should throw InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.equal(handleError);
      expect(handleAbciResponseErrorMock).to.be.calledOnceWith(abciResponseError);
    }
  });

  it('should throw error if fetchDocuments throws unknown error', async () => {
    const error = new Error('Some error');

    driveStateRepositoryMock.fetchDocuments.throws(error);

    try {
      await getDocumentsHandler(call);

      expect.fail('should throw InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(handleAbciResponseErrorMock).to.not.be.called();
    }
  });
});
