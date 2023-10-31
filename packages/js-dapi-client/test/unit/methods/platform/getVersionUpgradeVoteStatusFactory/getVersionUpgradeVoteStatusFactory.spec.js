const {
  v0: {
    PlatformPromiseClient,
    GetVersionUpgradeVoteStatusRequest,
    GetVersionUpgradeVoteStatusResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const getVersionUpgradeVoteStatusFactory = require('../../../../../lib/methods/platform/getVersionUpgradeVoteStatus/getVersionUpgradeVoteStatusFactory');
const VersionSignal = require('../../../../../lib/methods/platform/getVersionUpgradeVoteStatus/VersionSignal');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getVersionUpgradeVoteStatusFactory', () => {
  let grpcTransportMock;
  let getVersionUpgradeVoteStatus;
  let options;
  let response;
  let versionSignalFixture;
  let metadataFixture;
  let proofFixture;
  let proofResponse;
  let startProTxHash;

  beforeEach(async function beforeEach() {
    startProTxHash = Buffer.alloc(32).fill('a').toString('hex');
    versionSignalFixture = new VersionSignal(Buffer.alloc(32).toString('hex'), 1);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { GetVersionUpgradeVoteStatusResponseV0 } = GetVersionUpgradeVoteStatusResponse;
    const {
      VersionSignal: VersionSignalProto,
      VersionSignals,
    } = GetVersionUpgradeVoteStatusResponseV0;
    response = new GetVersionUpgradeVoteStatusResponse();
    response.setV0(
      new GetVersionUpgradeVoteStatusResponseV0()
        .setVersions(new VersionSignals()
          .setVersionSignalsList([new VersionSignalProto()
            .setProTxHash(versionSignalFixture.getProTxHash())
            .setVersion(versionSignalFixture.getVersion())]))
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

    getVersionUpgradeVoteStatus = getVersionUpgradeVoteStatusFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return epochs info', async () => {
    const result = await getVersionUpgradeVoteStatus(startProTxHash, 1, options);

    const { GetVersionUpgradeVoteStatusRequestV0 } = GetVersionUpgradeVoteStatusRequest;
    const request = new GetVersionUpgradeVoteStatusRequest();
    request.setV0(
      new GetVersionUpgradeVoteStatusRequestV0()
        .setStartProTxHash(Buffer.from(startProTxHash, 'hex'))
        .setCount(1)
        .setProve(!!options.prove),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getVersionUpgradeVoteStatus',
      request,
      options,
    );

    expect(result.getVersionSignals()).to.deep.equal([versionSignalFixture]);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    options.ascending = true;
    response.getV0().setVersions(undefined);
    response.getV0().setProof(proofResponse);

    const result = await getVersionUpgradeVoteStatus(startProTxHash, 1, options);

    const { GetVersionUpgradeVoteStatusRequestV0 } = GetVersionUpgradeVoteStatusRequest;
    const request = new GetVersionUpgradeVoteStatusRequest();
    request.setV0(
      new GetVersionUpgradeVoteStatusRequestV0()
        .setStartProTxHash(Buffer.from(startProTxHash, 'hex'))
        .setCount(1)
        .setProve(!!options.ascending),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getVersionUpgradeVoteStatus',
      request,
      options,
    );

    expect(result.getVersionSignals()).to.deep.equal([]);

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

    const { GetVersionUpgradeVoteStatusRequestV0 } = GetVersionUpgradeVoteStatusRequest;
    const request = new GetVersionUpgradeVoteStatusRequest();
    request.setV0(
      new GetVersionUpgradeVoteStatusRequestV0()
        .setStartProTxHash(Buffer.from(startProTxHash, 'hex'))
        .setCount(1)
        .setProve(!!options.ascending),
    );

    try {
      await getVersionUpgradeVoteStatus(startProTxHash, 1, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getVersionUpgradeVoteStatus',
        request,
        options,
      );
    }
  });
});
