const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetDataContractResponse,
    Proof,
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getDataContractHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getDataContractHandlerFactory',
);

const AbciResponseError = require('../../../../../lib/errors/AbciResponseError');

describe('getDataContractHandlerFactory', () => {
  let call;
  let getDataContractHandler;
  let driveStateRepositoryMock;
  let request;
  let id;
  let dataContractFixture;
  let handleAbciResponseErrorMock;
  let proofFixture;
  let proofMock;
  let response;
  let storeTreeProofs;

  beforeEach(function beforeEach() {
    id = generateRandomIdentifier();
    request = {
      getId: this.sinon.stub().returns(id),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, request);

    dataContractFixture = getDataContractFixture();
    proofFixture = {
      rootTreeProof: Buffer.alloc(1, 1),
      storeTreeProof: Buffer.alloc(1, 2),
    };

    storeTreeProofs = new StoreTreeProofs();
    storeTreeProofs.setDataContractsProof(proofFixture.storeTreeProof);

    proofMock = new Proof();
    proofMock.setRootTreeProof(proofFixture.rootTreeProof);
    proofMock.setStoreTreeProofs(storeTreeProofs);

    response = new GetDataContractResponse();
    response.setProof(proofMock);
    response.setDataContract(dataContractFixture.toBuffer());

    driveStateRepositoryMock = {
      fetchDataContract: this.sinon.stub().resolves(response.serializeBinary()),
    };

    handleAbciResponseErrorMock = this.sinon.stub();

    getDataContractHandler = getDataContractHandlerFactory(
      driveStateRepositoryMock,
      handleAbciResponseErrorMock,
    );
  });

  it('should return valid data', async () => {
    const result = await getDataContractHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractResponse);

    const contractBinary = result.getDataContract();
    expect(contractBinary).to.be.an.instanceOf(Uint8Array);

    expect(handleAbciResponseErrorMock).to.not.be.called();

    expect(contractBinary).to.deep.equal(dataContractFixture.toBuffer());

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const rootTreeProof = proof.getRootTreeProof();
    const resultStoreTreeProofs = proof.getStoreTreeProofs();

    expect(rootTreeProof).to.deep.equal(proofFixture.rootTreeProof);
    expect(resultStoreTreeProofs).to.deep.equal(storeTreeProofs);

    expect(driveStateRepositoryMock.fetchDataContract).to.be.calledOnceWith(id, true);
  });

  it('should not include proof', async () => {
    request.getProve.returns(false);
    response.setProof(null);
    driveStateRepositoryMock.fetchDataContract.resolves(response.serializeBinary());

    const result = await getDataContractHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractResponse);
    const proof = result.getProof();

    expect(proof).to.be.undefined();

    expect(driveStateRepositoryMock.fetchDataContract).to.be.calledOnceWith(id, false);
  });

  it('should throw InvalidArgumentGrpcError error if id is not specified', async () => {
    id = null;
    request.getId.returns(id);

    try {
      await getDataContractHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('id is not specified');
      expect(driveStateRepositoryMock.fetchDataContract).to.be.not.called();
      expect(handleAbciResponseErrorMock).to.be.not.called();
    }
  });

  it('should throw InvalidArgumentGrpcError if driveStateRepository throws AbciResponseError', async () => {
    const code = 2;
    const message = 'Some error';
    const data = 42;
    const abciResponseError = new AbciResponseError(code, { message, data });

    const handleError = new InvalidArgumentGrpcError('Another error');

    handleAbciResponseErrorMock.throws(handleError);

    driveStateRepositoryMock.fetchDataContract.throws(abciResponseError);

    try {
      await getDataContractHandler(call);

      expect.fail('should throw InvalidArgumentGrpcError');
    } catch (e) {
      expect(e).to.equal(handleError);
      expect(handleAbciResponseErrorMock).to.be.calledOnceWith(abciResponseError);
    }
  });

  it('should throw error if driveStateRepository throws unknown error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveStateRepositoryMock.fetchDataContract.throws(abciResponseError);

    try {
      await getDataContractHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
      expect(handleAbciResponseErrorMock).to.be.not.called();
    }
  });
});
