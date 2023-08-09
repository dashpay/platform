const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityBalanceResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentityBalanceHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityBalanceHandlerFactory');

describe('getIdentityBalanceHandlerFactory', () => {
  let call;
  let getIdentityBalanceHandler;
  let driveClientMock;
  let request;
  let id;
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

    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    response = new GetIdentityBalanceResponse();
    response.setBalance({
      toArray: () => [],
      getValue: () => 0,
    });

    proofResponse = new GetIdentityBalanceResponse();
    proofResponse.setProof(proofMock);

    driveClientMock = {
      fetchIdentityBalance: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentityBalanceHandler = getIdentityBalanceHandlerFactory(driveClientMock);
  });

  it('should return identity balance', async () => {
    const result = await getIdentityBalanceHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityBalanceResponse);

    const identityBalance = result.getBalance().getValue();

    expect(identityBalance).to.deep.equal(0);

    const proof = result.getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async function it() {
    driveClientMock = {
      fetchIdentityBalance: this.sinon.stub().resolves(proofResponse.serializeBinary()),
    };

    getIdentityBalanceHandler = getIdentityBalanceHandlerFactory(driveClientMock);

    const result = await getIdentityBalanceHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityBalanceResponse);

    const identityBalance = result.getBalance();
    expect(identityBalance).to.be.undefined();

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveClientMock.fetchIdentityBalance).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getId.returns(null);

    try {
      await getIdentityBalanceHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('identity id is not specified');
      expect(driveClientMock.fetchIdentityBalance).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveClientMock.fetchIdentityBalance.throws(abciResponseError);

    try {
      await getIdentityBalanceHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
