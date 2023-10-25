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
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getDataContractFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getDataContractHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getDataContractHandlerFactory',
);

describe('getDataContractHandlerFactory', () => {
  let call;
  let getDataContractHandler;
  let driveClientMock;
  let request;
  let id;
  let dataContractFixture;
  let proofFixture;
  let proofMock;
  let response;
  let proofResponse;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();
    request = {
      getId: this.sinon.stub().returns(id),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, request);

    dataContractFixture = await getDataContractFixture();
    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    response = new GetDataContractResponse();
    response.setV0(
      new GetDataContractResponse.GetDataContractResponseV0()
        .setDataContract(dataContractFixture.toBuffer()),
    );

    proofResponse = new GetDataContractResponse();
    proofResponse.setV0(
      new GetDataContractResponse.GetDataContractResponseV0()
        .setProof(proofMock),
    );

    driveClientMock = {
      fetchDataContract: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getDataContractHandler = getDataContractHandlerFactory(
      driveClientMock,
    );
  });

  it('should return data contract', async () => {
    const result = await getDataContractHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractResponse);

    const contractBinary = result.getV0().getDataContract();
    expect(contractBinary).to.be.an.instanceOf(Uint8Array);

    expect(contractBinary).to.deep.equal(dataContractFixture.toBuffer());

    const proof = result.getV0().getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async function it() {
    driveClientMock = {
      fetchDataContract: this.sinon.stub().resolves(proofResponse.serializeBinary()),
    };

    getDataContractHandler = getDataContractHandlerFactory(
      driveClientMock,
    );

    const result = await getDataContractHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractResponse);

    const contractBinary = result.getV0().getDataContract();
    expect(contractBinary).to.be.equal('');

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveClientMock.fetchDataContract).to.be.calledOnceWith(call.request);
  });

  it('should not include proof', async () => {
    request.getProve.returns(false);
    response.getV0().setProof(null);
    driveClientMock.fetchDataContract.resolves(response.serializeBinary());

    const result = await getDataContractHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractResponse);
    const proof = result.getV0().getProof();

    expect(proof).to.be.undefined();

    expect(driveClientMock.fetchDataContract).to.be.calledOnceWith(call.request);
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
      expect(driveClientMock.fetchDataContract).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveClientMock.fetchDataContract.throws(abciResponseError);

    try {
      await getDataContractHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
