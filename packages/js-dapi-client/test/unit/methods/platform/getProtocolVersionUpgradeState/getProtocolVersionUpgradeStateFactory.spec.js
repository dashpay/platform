import dapiGrpc from '@dashevo/dapi-grpc';

import getProtocolVersionUpgradeStateFactory from '../../../../../lib/methods/platform/getProtocolVersionUpgradeState/getProtocolVersionUpgradeStateFactory.js';
import VersionEntry from '../../../../../lib/methods/platform/getProtocolVersionUpgradeState/VersionEntry.js';
import getMetadataFixture from '../../../../../lib/test/fixtures/getMetadataFixture.js';
import getProofFixture from '../../../../../lib/test/fixtures/getProofFixture.js';
import Proof from '../../../../../lib/methods/platform/response/Proof.js';

const {
  v0: {
    PlatformPromiseClient,
    GetProtocolVersionUpgradeStateRequest,
    GetProtocolVersionUpgradeStateResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = dapiGrpc;

describe('getProtocolVersionUpgradeStateFactory', () => {
  let grpcTransportMock;
  let getProtocolVersionUpgradeState;
  let options;
  let response;
  let versionEntryFixture;
  let metadataFixture;
  let proofFixture;
  let proofResponse;

  beforeEach(async function beforeEach() {
    versionEntryFixture = new VersionEntry(1, 1);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { GetProtocolVersionUpgradeStateResponseV0 } = GetProtocolVersionUpgradeStateResponse;
    const {
      VersionEntry: VersionEntryProto,
      Versions,
    } = GetProtocolVersionUpgradeStateResponseV0;
    response = new GetProtocolVersionUpgradeStateResponse();
    response.setV0(
      new GetProtocolVersionUpgradeStateResponseV0()
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

    getProtocolVersionUpgradeState = getProtocolVersionUpgradeStateFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return version upgrade state', async () => {
    const result = await getProtocolVersionUpgradeState(options);

    const { GetProtocolVersionUpgradeStateRequestV0 } = GetProtocolVersionUpgradeStateRequest;
    const request = new GetProtocolVersionUpgradeStateRequest();
    request.setV0(
      new GetProtocolVersionUpgradeStateRequestV0()
        .setProve(!!options.prove),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getProtocolVersionUpgradeState',
      request,
      options,
    );

    expect(result.getVersionEntries()).to.deep.equal([versionEntryFixture]);

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

    const result = await getProtocolVersionUpgradeState(options);

    const { GetProtocolVersionUpgradeStateRequestV0 } = GetProtocolVersionUpgradeStateRequest;
    const request = new GetProtocolVersionUpgradeStateRequest();
    request.setV0(
      new GetProtocolVersionUpgradeStateRequestV0()
        .setProve(!!options.ascending),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getProtocolVersionUpgradeState',
      request,
      options,
    );

    expect(result.getVersionEntries()).to.deep.equal([]);

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

    const { GetProtocolVersionUpgradeStateRequestV0 } = GetProtocolVersionUpgradeStateRequest;
    const request = new GetProtocolVersionUpgradeStateRequest();
    request.setV0(
      new GetProtocolVersionUpgradeStateRequestV0()
        .setProve(!!options.ascending),
    );

    try {
      await getProtocolVersionUpgradeState(options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getProtocolVersionUpgradeState',
        request,
        options,
      );
    }
  });
});
