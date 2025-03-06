const {
  v0: {
    PlatformPromiseClient,
    GetIdentityNonceRequest,
    GetIdentityNonceResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityNonceFactory = require('../../../../../lib/methods/platform/getIdentityNonce/getIdentityNonceFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getIdentityNonceFactory', () => {
  let grpcTransportMock;
  let getIdentityNonce;
  let options;
  let response;
  let nonce;
  let identityId;
  let metadataFixture;
  let proofFixture;
  let proofResponse;

  beforeEach(async function beforeEach() {
    nonce = BigInt(1);
    identityId = Buffer.alloc(32).fill(0);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { GetIdentityNonceResponseV0 } = GetIdentityNonceResponse;
    response = new GetIdentityNonceResponse();
    response.setV0(
      new GetIdentityNonceResponseV0()
        .setIdentityNonce(nonce)
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

    getIdentityNonce = getIdentityNonceFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return identity nonce', async () => {
    const result = await getIdentityNonce(identityId, options);

    const { GetIdentityNonceRequestV0 } = GetIdentityNonceRequest;
    const request = new GetIdentityNonceRequest();
    request.setV0(
      new GetIdentityNonceRequestV0()
        .setIdentityId(identityId)
        .setProve(false),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityNonce',
      request,
      options,
    );
    expect(result.getIdentityNonce()).to.deep.equal(nonce);

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
    response.getV0().setIdentityNonce(undefined);
    response.getV0().setProof(proofResponse);

    const result = await getIdentityNonce(identityId, options);

    const { GetIdentityNonceRequestV0 } = GetIdentityNonceRequest;
    const request = new GetIdentityNonceRequest();
    request.setV0(
      new GetIdentityNonceRequestV0()
        .setIdentityId(identityId)
        .setProve(true),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityNonce',
      request,
      options,
    );

    expect(result.getIdentityNonce()).to.deep.equal(BigInt(0));

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

    const { GetIdentityNonceRequestV0 } = GetIdentityNonceRequest;
    const request = new GetIdentityNonceRequest();
    request.setV0(
      new GetIdentityNonceRequestV0()
        .setIdentityId(identityId)
        .setProve(false),
    );

    try {
      await getIdentityNonce(identityId, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentityNonce',
        request,
        options,
      );
    }
  });
});
