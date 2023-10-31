const {
  v0: {
    PlatformPromiseClient,
    GetVersionUpgradeStateRequest,
    GetVersionUpgradeStateResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const getVersionUpgradeStateFactory = require('../../../../../lib/methods/platform/getVersionUpgradeState/getVersionUpgradeStateFactory');
const VersionEntry = require('../../../../../lib/methods/platform/getVersionUpgradeState/VersionEntry');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getVersionUpgradeStateFactory', () => {
  let grpcTransportMock;
  let getVersionUpgradeState;
  let options;
  let response;
  let versionEntryFixture;
  let metadataFixture;
  let proofFixture;
  let proofResponse;
  let startProTxHash;

  beforeEach(async function beforeEach() {
    startProTxHash = Buffer.alloc(32).fill('a').toString('hex');
    versionEntryFixture = new VersionEntry(1, 1);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { GetVersionUpgradeStateResponseV0 } = GetVersionUpgradeStateResponse;
    const {
      VersionEntry: VersionEntryProto,
      Versions,
    } = GetVersionUpgradeStateResponseV0;
    response = new GetVersionUpgradeStateResponse();
    response.setV0(
      new GetVersionUpgradeStateResponseV0()
        .setVersions(new Versions()
          .setVersionsList([new VersionEntryProto()
            .setVersionNumber(versionEntryFixture.getVersionNumber())
            .setVoteCount(versionEntryFixture.getVoteCount())]))
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

    getVersionUpgradeState = getVersionUpgradeStateFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return version upgrade state', async () => {
    const result = await getVersionUpgradeState(options);

    const { GetVersionUpgradeStateRequestV0 } = GetVersionUpgradeStateRequest;
    const request = new GetVersionUpgradeStateRequest();
    request.setV0(
      new GetVersionUpgradeStateRequestV0()
        .setProve(!!options.prove),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getVersionUpgradeState',
      request,
      options,
    );

    expect(result.getVersionEntries()).to.deep.equal([versionEntryFixture]);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    options.ascending = true;
    response.getV0().setVersions(undefined);
    response.getV0().setProof(proofResponse);

    const result = await getVersionUpgradeState(options);

    const { GetVersionUpgradeStateRequestV0 } = GetVersionUpgradeStateRequest;
    const request = new GetVersionUpgradeStateRequest();
    request.setV0(
      new GetVersionUpgradeStateRequestV0()
        .setProve(!!options.ascending),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getVersionUpgradeState',
      request,
      options,
    );

    expect(result.getVersionEntries()).to.deep.equal([]);

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

    const { GetVersionUpgradeStateRequestV0 } = GetVersionUpgradeStateRequest;
    const request = new GetVersionUpgradeStateRequest();
    request.setV0(
      new GetVersionUpgradeStateRequestV0()
        .setProve(!!options.ascending),
    );

    try {
      await getVersionUpgradeState(options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getVersionUpgradeState',
        request,
        options,
      );
    }
  });
});
