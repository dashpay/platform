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
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

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
  let storeTreeProofs;

  beforeEach(function beforeEach() {
    dataContractId = generateRandomIdentifier();
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
      getProve: this.sinon.stub().returns(false),
    };

    call = new GrpcCallMock(this.sinon, request);

    const [document] = getDocumentsFixture();

    documentsFixture = [document];

    documentsSerialized = documentsFixture.map(documentItem => documentItem.toBuffer());
    proofFixture = {
      rootTreeProof: Buffer.alloc(1, 1),
      storeTreeProof: Buffer.alloc(1, 2),
    };

    storeTreeProofs = new StoreTreeProofs();
    storeTreeProofs.setDataContractsProof(proofFixture.storeTreeProof);

    proofMock = new Proof();
    proofMock.setRootTreeProof(proofFixture.rootTreeProof);
    proofMock.setStoreTreeProofs(storeTreeProofs);

    response = new GetDocumentsResponse();
    response.setProof(proofMock);
    response.setDocumentsList(documentsSerialized);

    driveStateRepositoryMock = {
      fetchDocuments: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getDocumentsHandler = getDocumentsHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid result', async () => {
    response.setProof(null);

    driveStateRepositoryMock.fetchDocuments.resolves(response.serializeBinary());

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
      false,
    );

    expect(documentsBinary[0]).to.deep.equal(documentsSerialized[0]);

    const proof = result.getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    request.getProve.returns(true);

    const result = await getDocumentsHandler(call);

    expect(result).to.be.an.instanceOf(GetDocumentsResponse);

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
      true,
    );

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const rootTreeProof = proof.getRootTreeProof();
    const resultStoreTreeProofs = proof.getStoreTreeProofs();

    expect(rootTreeProof).to.deep.equal(proofFixture.rootTreeProof);
    expect(resultStoreTreeProofs).to.deep.equal(storeTreeProofs);
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
