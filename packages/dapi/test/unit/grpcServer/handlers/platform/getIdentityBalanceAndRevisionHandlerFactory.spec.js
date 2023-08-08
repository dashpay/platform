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

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentityBalanceAndRevisionHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityBalanceAndRevisionHandlerFactory');

describe('getIdentityBalanceHandlerFactory', () => {
  let call;
  let getIdentityBalanceAndRevisionHandler;
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

    const revisionAndBalance = new GetIdentityBalanceAndRevisionResponse.BalanceAndRevision();
    revisionAndBalance.setRevision({
      toArray: () => [],
      getValue: () => 0,
    });
    revisionAndBalance.setBalance({
      toArray: () => [],
      getValue: () => 0,
    });

    response = new GetIdentityBalanceAndRevisionResponse();
    response.setBalanceAndRevision(revisionAndBalance);

    proofResponse = new GetIdentityBalanceAndRevisionResponse();
    proofResponse.setProof(proofMock);

    driveClientMock = {
      fetchIdentityBalanceAndRevision: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentityBalanceAndRevisionHandler = getIdentityBalanceAndRevisionHandlerFactory(
      driveClientMock,
    );
  });

  it('should return identity balance and revision', async () => {
    const result = await getIdentityBalanceAndRevisionHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityBalanceAndRevisionResponse);

    const identityRevisionAndBalance = result.getBalanceAndRevision();

    expect(identityRevisionAndBalance.getRevision().getValue()).to.deep.equal(0);
    expect(identityRevisionAndBalance.getBalance().getValue()).to.deep.equal(0);

    const proof = result.getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async function it() {
    driveClientMock = {
      fetchIdentityBalanceAndRevision: this.sinon.stub().resolves(
        proofResponse.serializeBinary(),
      ),
    };

    getIdentityBalanceAndRevisionHandler = getIdentityBalanceAndRevisionHandlerFactory(
      driveClientMock,
    );

    const result = await getIdentityBalanceAndRevisionHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityBalanceAndRevisionResponse);

    const identityRevisionAndBalance = result.getBalanceAndRevision();
    expect(identityRevisionAndBalance).to.be.undefined();

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveClientMock.fetchIdentityBalanceAndRevision).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getId.returns(null);

    try {
      await getIdentityBalanceAndRevisionHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('identity id is not specified');
      expect(driveClientMock.fetchIdentityBalanceAndRevision).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveClientMock.fetchIdentityBalanceAndRevision.throws(abciResponseError);

    try {
      await getIdentityBalanceAndRevisionHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
