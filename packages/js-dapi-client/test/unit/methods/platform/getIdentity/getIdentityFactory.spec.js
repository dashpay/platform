const {
  v0: {
    PlatformPromiseClient,
    GetIdentityRequest,
    GetIdentityResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

const getIdentityFactory = require('../../../../../lib/methods/platform/getIdentity/getIdentityFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getIdentityFactory', () => {
  let grpcTransportMock;
  let getIdentity;
  let options;
  let response;
  let identityFixture;
  let identityId;
  let metadataFixture;
  let proofFixture;
  let proofResponse;

  beforeEach(async function beforeEach() {
    identityFixture = await getIdentityFixture();
    identityId = identityFixture.getId();

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { GetIdentityResponseV0 } = GetIdentityResponse;
    response = new GetIdentityResponse();
    response.setV0(
      new GetIdentityResponseV0()
        .setIdentity(identityFixture.toBuffer())
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

    getIdentity = getIdentityFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return identity', async () => {
    const result = await getIdentity(identityId, options);

    const { GetIdentityRequestV0 } = GetIdentityRequest;
    const request = new GetIdentityRequest();
    request.setV0(
      new GetIdentityRequestV0()
        .setId(identityId.toBuffer())
        .setProve(false),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentity',
      request,
      options,
    );
    expect(result.getIdentity()).to.deep.equal(identityFixture.toBuffer());

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
    response.getV0().setIdentity(undefined);
    response.getV0().setProof(proofResponse);

    const result = await getIdentity(identityId, options);

    const { GetIdentityRequestV0 } = GetIdentityRequest;
    const request = new GetIdentityRequest();
    request.setV0(
      new GetIdentityRequestV0()
        .setId(identityId.toBuffer())
        .setProve(true),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentity',
      request,
      options,
    );

    expect(result.getIdentity()).to.deep.equal(Buffer.alloc(0));

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

    const { GetIdentityRequestV0 } = GetIdentityRequest;
    const request = new GetIdentityRequest();
    request.setV0(
      new GetIdentityRequestV0()
        .setId(identityId.toBuffer())
        .setProve(false),
    );

    try {
      await getIdentity(identityId, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentity',
        request,
        options,
      );
    }
  });
});
