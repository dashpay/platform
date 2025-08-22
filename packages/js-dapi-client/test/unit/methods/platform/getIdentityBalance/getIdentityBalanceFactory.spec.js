const {
  v0: {
    PlatformPromiseClient,
    GetIdentityBalanceRequest,
    GetIdentityBalanceResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const { GetIdentityBalanceResponseV0 } = GetIdentityBalanceResponse;

const getIdentityBalanceFactory = require('../../../../../lib/methods/platform/getIdentityBalance/getIdentityBalanceFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getIdentityBalanceFactory', () => {
  let grpcTransportMock;
  let getIdentityBalance;
  let options;
  let response;
  let balance;
  let identityId;
  let metadataFixture;
  let proofFixture;
  let proofResponse;

  beforeEach(async function beforeEach() {
    balance = BigInt(1337);

    identityId = Buffer.alloc(32).fill(0);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    response = new GetIdentityBalanceResponse();

    response.setV0(
      new GetIdentityBalanceResponseV0()
        .setBalance(balance)
        .setMetadata(metadata),
    );

    proofResponse = new ProofResponse();

    proofResponse.setQuorumHash(proofFixture.quorumHash);
    proofResponse.setSignature(proofFixture.signature);
    proofResponse.setGrovedbProof(proofFixture.merkleProof);
    proofResponse.setRound(proofFixture.round);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    getIdentityBalance = getIdentityBalanceFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return identity balance', async () => {
    const result = await getIdentityBalance(identityId, options);

    const { GetIdentityBalanceRequestV0 } = GetIdentityBalanceRequest;
    const request = new GetIdentityBalanceRequest();
    request.setV0(
      new GetIdentityBalanceRequestV0()
        .setId(identityId)
        .setProve(false),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityBalance',
      request,
      options,
    );
    expect(result.getBalance()).to.deep.equal(balance);

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    response.getV0().setBalance(undefined);
    response.getV0().setProof(proofResponse);

    const result = await getIdentityBalance(identityId, options);

    const { GetIdentityBalanceRequestV0 } = GetIdentityBalanceRequest;
    const request = new GetIdentityBalanceRequest();
    request.setV0(
      new GetIdentityBalanceRequestV0()
        .setId(identityId)
        .setProve(true),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityBalance',
      request,
      options,
    );

    expect(result.getBalance()).to.deep.equal(BigInt(0));

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(result.getProof()).to.be.an.instanceOf(Proof);
    expect(result.getProof().getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(result.getProof().getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(result.getProof().getSignature()).to.deep.equal(proofFixture.signature);
    expect(result.getProof().getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const { GetIdentityBalanceRequestV0 } = GetIdentityBalanceRequest;
    const request = new GetIdentityBalanceRequest();
    request.setV0(
      new GetIdentityBalanceRequestV0()
        .setId(identityId)
        .setProve(false),
    );

    try {
      await getIdentityBalance(identityId, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentityBalance',
        request,
        options,
      );
    }
  });
});
