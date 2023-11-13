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

const {
  GetIdentityBalanceResponseV0,
} = GetIdentityBalanceResponse;

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentityBalanceHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityBalanceHandlerFactory');

describe('getIdentityBalanceHandlerFactory', () => {
  let call;
  let getIdentityBalanceHandler;
  let request;
  let id;
  let fetchIdentityBalanceMock;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();
    request = {
      getId: this.sinon.stub().returns(id),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, {
      getV0: () => request,
    });

    fetchIdentityBalanceMock = this.sinon.stub();

    getIdentityBalanceHandler = getIdentityBalanceHandlerFactory({
      fetchIdentityBalance: fetchIdentityBalanceMock,
    });
  });

  it('should return identity balance', async () => {
    const response = new GetIdentityBalanceResponse()
      .setV0(new GetIdentityBalanceResponseV0().setBalance(15));

    fetchIdentityBalanceMock.resolves(response.serializeBinary());

    const result = await getIdentityBalanceHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityBalanceResponse);

    const identityBalance = result.getV0().getBalance();

    expect(identityBalance).to.deep.equal(15);
  });

  it('should return proof', async () => {
    const proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    const proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    const response = new GetIdentityBalanceResponse()
      .setV0(new GetIdentityBalanceResponseV0().setProof(proofMock));

    fetchIdentityBalanceMock.resolves(response.serializeBinary());

    const result = await getIdentityBalanceHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityBalanceResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(fetchIdentityBalanceMock).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getId.returns(null);

    try {
      await getIdentityBalanceHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('identity id is not specified');
      expect(fetchIdentityBalanceMock).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    fetchIdentityBalanceMock.throws(abciResponseError);

    try {
      await getIdentityBalanceHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
