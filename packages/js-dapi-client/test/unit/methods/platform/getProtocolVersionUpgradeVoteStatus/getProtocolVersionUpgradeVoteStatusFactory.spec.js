import dapiGrpc from '@dashevo/dapi-grpc';

import getProtocolVersionUpgradeVoteStatusFactory from '../../../../../lib/methods/platform/getProtocolVersionUpgradeVoteStatus/getProtocolVersionUpgradeVoteStatusFactory.js';
import VersionSignal from '../../../../../lib/methods/platform/getProtocolVersionUpgradeVoteStatus/VersionSignal.js';
import getMetadataFixture from '../../../../../lib/test/fixtures/getMetadataFixture.js';
import getProofFixture from '../../../../../lib/test/fixtures/getProofFixture.js';
import Proof from '../../../../../lib/methods/platform/response/Proof.js';
import { bytesToHex, hexToBytes } from '../../../../../lib/utils/bytes.js';

const {
  v0: {
    PlatformPromiseClient,
    GetProtocolVersionUpgradeVoteStatusRequest,
    GetProtocolVersionUpgradeVoteStatusResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = dapiGrpc;

describe('getProtocolVersionUpgradeVoteStatusFactory', () => {
  let grpcTransportMock;
  let getProtocolVersionUpgradeVoteStatus;
  let options;
  let response;
  let versionSignalFixture;
  let metadataFixture;
  let proofFixture;
  let proofResponse;
  let startProTxHash;

  beforeEach(async function beforeEach() {
    startProTxHash = bytesToHex(new Uint8Array(32).fill(0x61));
    versionSignalFixture = new VersionSignal(bytesToHex(new Uint8Array(32)), 1);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const {
      GetProtocolVersionUpgradeVoteStatusResponseV0,
    } = GetProtocolVersionUpgradeVoteStatusResponse;
    const {
      VersionSignal: VersionSignalProto,
      VersionSignals,
    } = GetProtocolVersionUpgradeVoteStatusResponseV0;
    response = new GetProtocolVersionUpgradeVoteStatusResponse();
    response.setV0(
      new GetProtocolVersionUpgradeVoteStatusResponseV0()
        .setVersions(new VersionSignals()
          .setVersionSignalsList([new VersionSignalProto()
            .setProTxHash(hexToBytes(versionSignalFixture.getProTxHash()))
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

    getProtocolVersionUpgradeVoteStatus = getProtocolVersionUpgradeVoteStatusFactory(
      grpcTransportMock,
    );

    options = {
      timeout: 1000,
    };
  });

  it('should return votes statuses', async () => {
    const result = await getProtocolVersionUpgradeVoteStatus(startProTxHash, 1, options);

    const {
      GetProtocolVersionUpgradeVoteStatusRequestV0,
    } = GetProtocolVersionUpgradeVoteStatusRequest;
    const request = new GetProtocolVersionUpgradeVoteStatusRequest();
    request.setV0(
      new GetProtocolVersionUpgradeVoteStatusRequestV0()
        .setStartProTxHash(hexToBytes(startProTxHash))
        .setCount(1)
        .setProve(!!options.prove),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getProtocolVersionUpgradeVoteStatus',
      request,
      options,
    );

    expect(result.getVersionSignals()).to.deep.equal([versionSignalFixture]);

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
    response.getV0().setVersions(undefined);
    response.getV0().setProof(proofResponse);

    const result = await getProtocolVersionUpgradeVoteStatus(startProTxHash, 1, options);

    const {
      GetProtocolVersionUpgradeVoteStatusRequestV0,
    } = GetProtocolVersionUpgradeVoteStatusRequest;
    const request = new GetProtocolVersionUpgradeVoteStatusRequest();
    request.setV0(
      new GetProtocolVersionUpgradeVoteStatusRequestV0()
        .setStartProTxHash(hexToBytes(startProTxHash))
        .setCount(1)
        .setProve(!!options.ascending),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getProtocolVersionUpgradeVoteStatus',
      request,
      options,
    );

    expect(result.getVersionSignals()).to.deep.equal([]);

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(result.getProof()).to.be.an.instanceOf(Proof);
    expect(result.getProof().getGrovedbProof()).to.deep.equal(new Uint8Array(proofFixture.merkleProof));
    expect(result.getProof().getQuorumHash()).to.deep.equal(new Uint8Array(proofFixture.quorumHash));
    expect(result.getProof().getSignature()).to.deep.equal(new Uint8Array(proofFixture.signature));
    expect(result.getProof().getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const {
      GetProtocolVersionUpgradeVoteStatusRequestV0,
    } = GetProtocolVersionUpgradeVoteStatusRequest;
    const request = new GetProtocolVersionUpgradeVoteStatusRequest();
    request.setV0(
      new GetProtocolVersionUpgradeVoteStatusRequestV0()
        .setStartProTxHash(hexToBytes(startProTxHash))
        .setCount(1)
        .setProve(!!options.ascending),
    );

    try {
      await getProtocolVersionUpgradeVoteStatus(startProTxHash, 1, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getProtocolVersionUpgradeVoteStatus',
        request,
        options,
      );
    }
  });
});
