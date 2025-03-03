const {
  v0: {
    PlatformPromiseClient,
    GetEpochsInfoRequest,
    GetEpochsInfoResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const { UInt32Value } = require('google-protobuf/google/protobuf/wrappers_pb');

const getEpochsInfoFactory = require('../../../../../lib/methods/platform/getEpochsInfo/getEpochsInfoFactory');
const EpochInfo = require('../../../../../lib/methods/platform/getEpochsInfo/EpochInfo');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getEpochsInfoFactory', () => {
  let grpcTransportMock;
  let getEpochsInfo;
  let options;
  let response;
  let epochInfoFixture;
  let metadataFixture;
  let proofFixture;
  let proofResponse;

  beforeEach(async function beforeEach() {
    epochInfoFixture = new EpochInfo(1, BigInt(1), 1, BigInt(Date.now()), 1.1);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { GetEpochsInfoResponseV0 } = GetEpochsInfoResponse;
    const { EpochInfo: EpochInfoProto, EpochInfos } = GetEpochsInfoResponseV0;
    response = new GetEpochsInfoResponse();
    response.setV0(
      new GetEpochsInfoResponseV0()
        .setEpochs(new EpochInfos()
          .setEpochInfosList([new EpochInfoProto()
            .setNumber(epochInfoFixture.getNumber())
            .setFirstBlockHeight(epochInfoFixture.getFirstBlockHeight())
            .setFirstCoreBlockHeight(epochInfoFixture.getFirstCoreBlockHeight())
            .setStartTime(epochInfoFixture.getStartTime())
            .setFeeMultiplier(epochInfoFixture.getFeeMultiplier())]))
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

    getEpochsInfo = getEpochsInfoFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return epochs info', async () => {
    const result = await getEpochsInfo(1, 1, options);

    const { GetEpochsInfoRequestV0 } = GetEpochsInfoRequest;
    const request = new GetEpochsInfoRequest();
    request.setV0(
      new GetEpochsInfoRequestV0()
        .setStartEpoch(new UInt32Value([1]))
        .setCount(1)
        .setAscending(!!options.ascending)
        .setProve(!!options.prove),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getEpochsInfo',
      request,
      options,
    );
    expect(result.getEpochsInfo()).to.deep.equal([epochInfoFixture]);

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
    options.ascending = true;
    response.getV0().setEpochs(undefined);
    response.getV0().setProof(proofResponse);

    const result = await getEpochsInfo(1, 1, options);

    const { GetEpochsInfoRequestV0 } = GetEpochsInfoRequest;
    const request = new GetEpochsInfoRequest();
    request.setV0(
      new GetEpochsInfoRequestV0()
        .setStartEpoch(new UInt32Value([1]))
        .setCount(1)
        .setAscending(!!options.ascending)
        .setProve(!!options.ascending),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getEpochsInfo',
      request,
      options,
    );

    expect(result.getEpochsInfo()).to.deep.equal([]);

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

    const { GetEpochsInfoRequestV0 } = GetEpochsInfoRequest;
    const request = new GetEpochsInfoRequest();
    request.setV0(
      new GetEpochsInfoRequestV0()
        .setStartEpoch(new UInt32Value([1]))
        .setCount(1)
        .setAscending(!!options.ascending)
        .setProve(!!options.ascending),
    );

    try {
      await getEpochsInfo(1, 1, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getEpochsInfo',
        request,
        options,
      );
    }
  });
});
