const { BytesValue } = require('google-protobuf/google/protobuf/wrappers_pb');
const {
  v0: {
    PlatformPromiseClient,
    GetIdentitiesByPublicKeyHashesRequest,
    GetIdentitiesByPublicKeyHashesResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');

const getIdentitiesByPublicKeyHashesFactory = require(
  '../../../../../lib/methods/platform/getIdentitiesByPublicKeyHashes/getIdentitiesByPublicKeyHashesFactory',
);
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getIdentitiesByPublicKeyHashesFactory', () => {
  let grpcTransportMock;
  let getIdentitiesByPublicKeyHashes;
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
      IdentitiesByPublicKeyHashes,
      PublicKeyHashIdentityEntry,
      GetIdentitiesByPublicKeyHashesResponseV0,
    } = GetIdentitiesByPublicKeyHashesResponse;

    response = new GetIdentitiesByPublicKeyHashesResponse();
    response.setV0(
      new GetIdentitiesByPublicKeyHashesResponseV0().setIdentities(
        new IdentitiesByPublicKeyHashes()
          .setIdentityEntriesList([
            new PublicKeyHashIdentityEntry()
              .setPublicKeyHash(publicKeyHash)
              .setValue(new BytesValue().setValue(identityFixture.toBuffer())),
          ]),
      ).setMetadata(metadata),
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

    getIdentitiesByPublicKeyHashes = getIdentitiesByPublicKeyHashesFactory(grpcTransportMock);
  });

  it('should return public key hashes to identity map', async () => {
    const result = await getIdentitiesByPublicKeyHashes([publicKeyHash], options);

    const { GetIdentitiesByPublicKeyHashesRequestV0 } = GetIdentitiesByPublicKeyHashesRequest;
    const request = new GetIdentitiesByPublicKeyHashesRequest();
    request.setV0(
      new GetIdentitiesByPublicKeyHashesRequestV0()
        .setPublicKeyHashesList([publicKeyHash])
        .setProve(false),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentitiesByPublicKeyHashes',
      request,
      options,
    );
    expect(result.getIdentities()).to.have.deep.equal([identityFixture.toBuffer()]);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    response.getV0().setProof(proofResponse);

    const result = await getIdentitiesByPublicKeyHashes([publicKeyHash], options);

    const { GetIdentitiesByPublicKeyHashesRequestV0 } = GetIdentitiesByPublicKeyHashesRequest;
    const request = new GetIdentitiesByPublicKeyHashesRequest();
    request.setV0(
      new GetIdentitiesByPublicKeyHashesRequestV0()
        .setPublicKeyHashesList([publicKeyHash])
        .setProve(true),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentitiesByPublicKeyHashes',
      request,
      options,
    );
    expect(result.getIdentities()).to.have.deep.members([]);

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

    const { GetIdentitiesByPublicKeyHashesRequestV0 } = GetIdentitiesByPublicKeyHashesRequest;
    const request = new GetIdentitiesByPublicKeyHashesRequest();
    request.setV0(
      new GetIdentitiesByPublicKeyHashesRequestV0()
        .setPublicKeyHashesList([publicKeyHash])
        .setProve(false),
    );

    try {
      await getIdentitiesByPublicKeyHashes([publicKeyHash], options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentitiesByPublicKeyHashes',
        request,
        options,
      );
    }
  });
});
