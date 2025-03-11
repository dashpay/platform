const {
  v0: {
    PlatformPromiseClient,
    GetIdentityKeysRequest,
    GetIdentityKeysResponse,
    ResponseMetadata,
    Proof: ProofResponse,
    KeyRequestType,
    SpecificKeys,
    AllKeys,
  },
} = require('@dashevo/dapi-grpc');
const { UInt32Value } = require('google-protobuf/google/protobuf/wrappers_pb');

const { GetIdentityKeysResponseV0 } = GetIdentityKeysResponse;
const { Keys } = GetIdentityKeysResponseV0;

const getIdentityKeysFactory = require('../../../../../lib/methods/platform/getIdentityKeys/getIdentityKeysFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getIdentityKeysFactory', () => {
  let grpcTransportMock;
  let getIdentityKeys;
  let options;
  let response;
  let metadata;
  let keys;
  let identityId;
  let keyIds;
  let limit;
  let metadataFixture;
  let proofFixture;
  let proofResponse;

  beforeEach(async function beforeEach() {
    keys = [Buffer.alloc(41), Buffer.alloc(46)];
    keyIds = [0, 1];
    limit = 100;

    identityId = Buffer.alloc(32).fill(0);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    response = new GetIdentityKeysResponse();

    response.setV0(
      new GetIdentityKeysResponseV0()
        .setKeys(new Keys().setKeysBytesList(keys))
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

    getIdentityKeys = getIdentityKeysFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return specific identity keys', async () => {
    response.setV0(
      new GetIdentityKeysResponseV0()
        .setKeys(new Keys().setKeysBytesList([keys[0]]))
        .setMetadata(metadata),
    );

    const result = await getIdentityKeys(identityId, [keyIds[0]], limit, options);

    const { GetIdentityKeysRequestV0 } = GetIdentityKeysRequest;
    const request = new GetIdentityKeysRequest();
    request.setV0(
      new GetIdentityKeysRequestV0()
        .setIdentityId(identityId)
        .setRequestType(new KeyRequestType().setSpecificKeys(new SpecificKeys()
          .setKeyIdsList([keyIds[0]])))
        .setLimit(new UInt32Value([limit]))
        .setProve(false),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityKeys',
      request,
      options,
    );
    expect(result.getIdentityKeys()).to.deep.equal([keys[0]]);
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

  it('should return all identity keys', async () => {
    response.setV0(
      new GetIdentityKeysResponseV0()
        .setKeys(new Keys().setKeysBytesList(keys))
        .setMetadata(metadata),
    );

    const result = await getIdentityKeys(identityId, null, limit, options);

    const { GetIdentityKeysRequestV0 } = GetIdentityKeysRequest;
    const request = new GetIdentityKeysRequest();
    request.setV0(
      new GetIdentityKeysRequestV0()
        .setIdentityId(identityId)
        .setRequestType(new KeyRequestType().setAllKeys(new AllKeys()))
        .setLimit(new UInt32Value([limit]))
        .setProve(false),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityKeys',
      request,
      options,
    );
    expect(result.getIdentityKeys()).to.deep.equal(keys);

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
    response.getV0().setKeys(undefined);
    response.getV0().setProof(proofResponse);

    const result = await getIdentityKeys(identityId, keyIds, limit, options);

    const { GetIdentityKeysRequestV0 } = GetIdentityKeysRequest;
    const request = new GetIdentityKeysRequest();
    request.setV0(
      new GetIdentityKeysRequestV0()
        .setIdentityId(identityId)
        .setRequestType(new KeyRequestType().setSpecificKeys(new SpecificKeys()
          .setKeyIdsList(keyIds)))
        .setLimit(new UInt32Value([limit]))
        .setProve(true),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityKeys',
      request,
      options,
    );

    expect(result.getIdentityKeys()).to.deep.equal([]);

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

    const { GetIdentityKeysRequestV0 } = GetIdentityKeysRequest;
    const request = new GetIdentityKeysRequest();
    request.setV0(
      new GetIdentityKeysRequestV0()
        .setIdentityId(identityId)
        .setRequestType(new KeyRequestType().setSpecificKeys(new SpecificKeys()
          .setKeyIdsList(keyIds)))
        .setLimit(new UInt32Value([100]))
        .setProve(false),
    );

    try {
      await getIdentityKeys(identityId, keyIds, limit, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentityKeys',
        request,
        options,
      );
    }
  });
});
