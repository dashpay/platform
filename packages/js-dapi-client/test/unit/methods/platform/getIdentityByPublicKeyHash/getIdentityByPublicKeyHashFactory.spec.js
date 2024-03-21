const {
  v0: {
    PlatformPromiseClient,
    GetIdentityByPublicKeyHashRequest,
    GetIdentityByPublicKeyHashResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');

const getIdentityByPublicKeyHashFactory = require(
  '../../../../../lib/methods/platform/getIdentityByPublicKeyHash/getIdentityByPublicKeyHashFactory',
);
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getIdentityByPublicKeyHashFactory', () => {
  let grpcTransportMock;
  let getIdentityByPublicKeyHash;
  let options;
  let response;
  let identityFixture;
  let publicKeyHash;
  let metadataFixture;
  let proofFixture;
  let proofResponse;

  beforeEach(async function beforeEach() {
    identityFixture = await getIdentityFixture();
    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const {
      GetIdentityByPublicKeyHashResponseV0,
    } = GetIdentityByPublicKeyHashResponse;

    response = new GetIdentityByPublicKeyHashResponse();
    response.setV0(
      new GetIdentityByPublicKeyHashResponseV0()
        .setIdentity(identityFixture.toBuffer())
        .setMetadata(metadata),
    );

    proofResponse = new ProofResponse();

    proofResponse.setQuorumHash(proofFixture.quorumHash);
    proofResponse.setSignature(proofFixture.signature);
    proofResponse.setGrovedbProof(proofFixture.merkleProof);
    proofResponse.setRound(proofFixture.round);

    publicKeyHash = identityFixture.getPublicKeyById(1).hash();

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
    };

    getIdentityByPublicKeyHash = getIdentityByPublicKeyHashFactory(grpcTransportMock);
  });

  it('should return identity', async () => {
    const result = await getIdentityByPublicKeyHash(publicKeyHash, options);

    const { GetIdentityByPublicKeyHashRequestV0 } = GetIdentityByPublicKeyHashRequest;
    const request = new GetIdentityByPublicKeyHashRequest();
    request.setV0(
      new GetIdentityByPublicKeyHashRequestV0()
        .setPublicKeyHash(publicKeyHash)
        .setProve(false),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityByPublicKeyHash',
      request,
      options,
    );
    expect(result.getIdentity()).to.have.deep.equal(identityFixture.toBuffer());
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    response.getV0().setProof(proofResponse);

    const result = await getIdentityByPublicKeyHash(publicKeyHash, options);

    const { GetIdentityByPublicKeyHashRequestV0 } = GetIdentityByPublicKeyHashRequest;
    const request = new GetIdentityByPublicKeyHashRequest();
    request.setV0(
      new GetIdentityByPublicKeyHashRequestV0()
        .setPublicKeyHash(publicKeyHash)
        .setProve(true),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityByPublicKeyHash',
      request,
      options,
    );
    expect(result.getIdentity()).to.deep.equal(Buffer.alloc(0));

    expect(result.getMetadata()).to.deep.equal(metadataFixture);

    expect(result.getProof()).to.be.an.instanceOf(Proof);
    expect(result.getProof().getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(result.getProof().getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(result.getProof().getSignature()).to.deep.equal(proofFixture.signature);
    expect(result.getProof().getRound()).to.deep.equal(proofFixture.round);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getMetadata().getHeight()).to.equal(metadataFixture.height);
    expect(result.getMetadata().getCoreChainLockedHeight()).to.equal(
      metadataFixture.coreChainLockedHeight,
    );
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const { GetIdentityByPublicKeyHashRequestV0 } = GetIdentityByPublicKeyHashRequest;
    const request = new GetIdentityByPublicKeyHashRequest();
    request.setV0(
      new GetIdentityByPublicKeyHashRequestV0()
        .setPublicKeyHash(publicKeyHash)
        .setProve(false),
    );

    try {
      await getIdentityByPublicKeyHash(publicKeyHash, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentityByPublicKeyHash',
        request,
        options,
      );
    }
  });
});
