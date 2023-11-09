const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityBalanceAndRevisionResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const {
  GetIdentityBalanceAndRevisionResponseV0,
} = GetIdentityBalanceAndRevisionResponse;

const {
  BalanceAndRevision,
} = GetIdentityBalanceAndRevisionResponseV0;

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentityBalanceAndRevisionHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityBalanceAndRevisionHandlerFactory');

describe('getIdentityBalanceAndRevisionHandlerFactory', () => {
  let call;
  let getIdentityBalanceAndRevisionHandler;
  let fetchIdentityBalanceAndRevisionMock;
  let request;
  let id;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();

    request = {
      getId: this.sinon.stub().returns(id),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, request);

    fetchIdentityBalanceAndRevisionMock = this.sinon.stub();

    getIdentityBalanceAndRevisionHandler = getIdentityBalanceAndRevisionHandlerFactory({
      fetchIdentityBalanceAndRevision: fetchIdentityBalanceAndRevisionMock,
    });
  });

  it('should return identity balance and revision', async () => {
    const revisionAndBalance = new BalanceAndRevision();

    revisionAndBalance.setRevision(1);
    revisionAndBalance.setBalance(15);

    const response = new GetIdentityBalanceAndRevisionResponse()
      .setV0(
        new GetIdentityBalanceAndRevisionResponseV0().setBalanceAndRevision(revisionAndBalance),
      );

    fetchIdentityBalanceAndRevisionMock.resolves(response.serializeBinary());

    const result = await getIdentityBalanceAndRevisionHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityBalanceAndRevisionResponse);

    const identityRevisionAndBalance = result.getV0().getBalanceAndRevision();

    expect(identityRevisionAndBalance.getRevision()).to.equals(1);
    expect(identityRevisionAndBalance.getBalance()).to.equals(15);

    expect(fetchIdentityBalanceAndRevisionMock).to.be.calledOnceWith(call.request);
  });

  it('should return proof', async () => {
    const proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    const proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    const response = new GetIdentityBalanceAndRevisionResponse()
      .setV0(new GetIdentityBalanceAndRevisionResponseV0().setProof(proofMock));

    fetchIdentityBalanceAndRevisionMock.resolves(response.serializeBinary());

    const result = await getIdentityBalanceAndRevisionHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityBalanceAndRevisionResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(fetchIdentityBalanceAndRevisionMock).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getId.returns(null);

    try {
      await getIdentityBalanceAndRevisionHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('identity id is not specified');
      expect(fetchIdentityBalanceAndRevisionMock).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    fetchIdentityBalanceAndRevisionMock.throws(abciResponseError);

    try {
      await getIdentityBalanceAndRevisionHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
