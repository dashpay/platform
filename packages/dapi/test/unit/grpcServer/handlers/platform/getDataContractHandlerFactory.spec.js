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
  let driveStateRepositoryMock;
  let request;
  let id;
  let dataContractFixture;
  let proofFixture;
  let proofMock;
  let response;

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
    proofMock.setMerkleProof(proofFixture.merkleProof);

    response = new GetDataContractResponse();
    response.setProof(proofMock);
    // TODO: Identifier/buffer issue - problem with Buffer shim:
    //  Without Buffer.from it throws AssertionError: Failure: Type not convertible to Uint8Array.
    response.setDataContract(Buffer.from(dataContractFixture.toBuffer()));

    driveStateRepositoryMock = {
      fetchDataContract: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getDataContractHandler = getDataContractHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid data', async () => {
    const result = await getDataContractHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractResponse);

    const contractBinary = result.getDataContract();
    expect(contractBinary).to.be.an.instanceOf(Uint8Array);

    expect(contractBinary).to.deep.equal(dataContractFixture.toBuffer());

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getMerkleProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveStateRepositoryMock.fetchDataContract).to.be.calledOnceWith(
      this.sinon.match((identifier) => identifier.equals(id.toBuffer())),
      true,
    );
  });

  it('should not include proof', async () => {
    request.getProve.returns(false);
    response.setProof(null);
    driveStateRepositoryMock.fetchDataContract.resolves(response.serializeBinary());

    const result = await getDataContractHandler(call);

    expect(result).to.be.an.instanceOf(GetDataContractResponse);
    const proof = result.getProof();

    expect(proof).to.be.undefined();

    expect(driveStateRepositoryMock.fetchDataContract).to.be.calledOnceWith(
      this.sinon.match((identifier) => identifier.equals(id.toBuffer())),
      false,
    );
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
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveStateRepositoryMock.fetchDataContract.throws(abciResponseError);

    try {
      await getDataContractHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
