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
    Proof,
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getDocumentsFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDocumentsFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getDocumentsHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getDocumentsHandlerFactory',
);

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
  let documentsSerialized;
  let proofFixture;
  let response;
  let proofMock;
  let proofResponse;

  beforeEach(async function beforeEach() {
    dataContractId = await generateRandomIdentifierAsync();
    documentType = 'document';
    where = [['name', '==', 'John']];
    orderBy = [{ order: 'asc' }];
    limit = 20;
    startAfter = new Uint8Array((await generateRandomIdentifierAsync()).toBuffer());
    startAt = new Uint8Array([]);

    request = {
      getDataContractId: this.sinon.stub().returns(dataContractId),
      getDocumentType: this.sinon.stub().returns(documentType),
      getWhere_asU8: this.sinon.stub().returns(new Uint8Array(cbor.encode(where))),
      getOrderBy_asU8: this.sinon.stub().returns(new Uint8Array(cbor.encode(orderBy))),
      getLimit: this.sinon.stub().returns(limit),
      getStartAfter_asU8: this.sinon.stub().returns(startAfter),
      getStartAt_asU8: this.sinon.stub().returns(startAt),
      getProve: this.sinon.stub().returns(false),
    };

    call = new GrpcCallMock(this.sinon, {
      getV0: () => request,
    });

    const [document] = await getDocumentsFixture();

    documentsFixture = [document];

    documentsSerialized = documentsFixture.map((documentItem) => documentItem.toBuffer());
    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    const { GetDocumentsResponseV0 } = GetDocumentsResponse;
    response = new GetDocumentsResponse();
    // const documentsList = new GetDocumentsResponse.Documents();
    // documentsList.setDocumentsList(documentsSerialized);
    response.setV0(
      new GetDocumentsResponseV0()
        .setDocuments(
          new GetDocumentsResponseV0.Documents()
            .setDocumentsList(documentsSerialized),
        ),
    );

    proofResponse = new GetDocumentsResponse();
    proofResponse.setV0(
      new GetDocumentsResponseV0().setProof(proofMock),
    );

    driveStateRepositoryMock = {
      fetchDocuments: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getDocumentsHandler = getDocumentsHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid result', async () => {
    response.getV0().setProof(null);

    const result = await getDocumentsHandler(call);

    expect(result).to.be.an.instanceOf(GetDocumentsResponse);
    const documents = result.getV0().getDocuments();

    const documentsBinary = documents.getDocumentsList();
    expect(documentsBinary).to.be.an('array');
    expect(documentsBinary).to.have.lengthOf(documentsFixture.length);

    expect(driveStateRepositoryMock.fetchDocuments).to.be.calledOnceWith(
      call.request,
    );

    expect(documentsBinary[0]).to.deep.equal(documentsSerialized[0]);

    const proof = result.getV0().getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    request.getProve.returns(true);
    driveStateRepositoryMock.fetchDocuments.resolves(proofResponse.serializeBinary());

    const result = await getDocumentsHandler(call);

    expect(result).to.be.an.instanceOf(GetDocumentsResponse);

    expect(driveStateRepositoryMock.fetchDocuments).to.be.calledOnceWith(
      call.request,
    );

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);
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
    }
  });

  it('should throw error if fetchDocuments throws an error', async () => {
    const error = new Error('Some error');

    driveStateRepositoryMock.fetchDocuments.throws(error);

    try {
      await getDocumentsHandler(call);

      expect.fail('should throw InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.equal(error);
    }
  });
});
