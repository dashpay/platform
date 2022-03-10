const {
  v0: {
    PlatformPromiseClient,
    GetIdentityRequest,
    GetIdentityResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

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

  beforeEach(function beforeEach() {
    identityFixture = getIdentityFixture();
    identityId = identityFixture.getId();

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    response = new GetIdentityResponse();
    response.setIdentity(identityFixture.toBuffer());
    response.setMetadata(metadata);

    proofResponse = new ProofResponse();

    proofResponse.setSignatureLlmqHash(proofFixture.signatureLLMQHash);
    proofResponse.setSignature(proofFixture.signature);
    proofResponse.setMerkleProof(proofFixture.merkleProof);

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

    const request = new GetIdentityRequest();
    request.setId(identityId.toBuffer());
    request.setProve(false);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentity',
      request,
      options,
    );
    expect(result.getIdentity()).to.deep.equal(identityFixture.toBuffer());
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    response.setIdentity(undefined);
    response.setProof(proofResponse);

    const result = await getIdentity(identityId, options);

    const request = new GetIdentityRequest();
    request.setId(identityId.toBuffer());
    request.setProve(true);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentity',
      request,
      options,
    );

    expect(result.getIdentity()).to.deep.equal(Buffer.alloc(0));

    expect(result.getMetadata()).to.deep.equal(metadataFixture);

    expect(result.getProof()).to.be.an.instanceOf(Proof);
    expect(result.getProof().getMerkleProof()).to.deep.equal(proofFixture.merkleProof);
    expect(result.getProof().getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(result.getProof().getSignature()).to.deep.equal(proofFixture.signature);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getMetadata().getHeight()).to.equal(metadataFixture.height);
    expect(result.getMetadata().getCoreChainLockedHeight()).to.equal(
      metadataFixture.coreChainLockedHeight,
    );
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const request = new GetIdentityRequest();
    request.setId(identityId.toBuffer());
    request.setProve(false);

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
