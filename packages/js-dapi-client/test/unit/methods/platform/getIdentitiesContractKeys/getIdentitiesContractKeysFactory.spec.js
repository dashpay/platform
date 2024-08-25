const {
  v0: {
    PlatformPromiseClient,
    GetIdentitiesContractKeysRequest,
    GetIdentitiesContractKeysResponse,
    KeyPurpose,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');

const getIdentitiesContractKeysFactory = require(
  '../../../../../lib/methods/platform/getIdentitiesContractKeys/getIdentitiesContractKeysFactory',
);
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getIdentitiesContractKeysFactory', () => {
  let grpcTransportMock;
  let getIdentitiesContractKeys;
  let options;
  let response;

  let identityFixtureA;
  let identityFixtureB;
  let contractId;
  let identitiesContractKeys;

  let metadataFixture;
  let proofFixture;
  let proofResponse;

  let mockRequest;

  beforeEach(async function beforeEach() {
    identityFixtureA = await getIdentityFixture(await generateRandomIdentifier());
    identityFixtureB = await getIdentityFixture(await generateRandomIdentifier());
    contractId = await generateRandomIdentifier();
    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const {
      GetIdentitiesContractKeysResponseV0,
    } = GetIdentitiesContractKeysResponse;

    const { IdentitiesKeys, IdentityKeys, PurposeKeys } = GetIdentitiesContractKeysResponseV0;

    response = new GetIdentitiesContractKeysResponse();
    response.setV0(
      new GetIdentitiesContractKeysResponseV0()
        .setIdentitiesKeys(new IdentitiesKeys()
          .setEntriesList([
            new IdentityKeys()
              .setIdentityId(new Uint8Array(identityFixtureA.getId().toBuffer()))
              .setKeysList([
                new PurposeKeys()
                  .setPurpose(KeyPurpose.ENCRYPTION)
                  .setKeysBytesList(identityFixtureA.getPublicKeys()
                    .map((key) => new Uint8Array(key.toBuffer()))),
              ]),
            new IdentityKeys()
              .setIdentityId(new Uint8Array(identityFixtureB.getId().toBuffer()))
              .setKeysList([
                new PurposeKeys()
                  .setPurpose(KeyPurpose.DECRYPTION)
                  .setKeysBytesList(identityFixtureB.getPublicKeys()
                    .map((key) => new Uint8Array(key.toBuffer()))),
              ]),
          ]))
        .setMetadata(metadata),
    );

    proofResponse = new ProofResponse();

    proofResponse.setQuorumHash(proofFixture.quorumHash);
    proofResponse.setSignature(proofFixture.signature);
    proofResponse.setGrovedbProof(proofFixture.merkleProof);
    proofResponse.setRound(proofFixture.round);

    identitiesContractKeys = {
      [identityFixtureA.getId().toString()]: {
        [KeyPurpose.ENCRYPTION]: identityFixtureA.getPublicKeys()
          .map((key) => new Uint8Array(key.toBuffer())),
      },
      [identityFixtureB.getId().toString()]: {
        [KeyPurpose.DECRYPTION]: identityFixtureB.getPublicKeys()
          .map((key) => new Uint8Array(key.toBuffer())),
      },
    };

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
    };

    mockRequest = () => {
      const { GetIdentitiesContractKeysRequestV0 } = GetIdentitiesContractKeysRequest;
      const request = new GetIdentitiesContractKeysRequest();
      request.setV0(
        new GetIdentitiesContractKeysRequestV0()
          .setProve(!!options.prove)
          .setIdentitiesIdsList(
            [Buffer.from(identityFixtureA.getId()), Buffer.from(identityFixtureB.getId())],
          )
          .setContractId(Buffer.from(contractId))
          .setPurposesList([KeyPurpose.ENCRYPTION, KeyPurpose.DECRYPTION])
          .setDocumentTypeName('contactRequest'),
      );

      return request;
    };

    getIdentitiesContractKeys = getIdentitiesContractKeysFactory(grpcTransportMock);
  });

  it('should return identity ids to key purposes to keys', async () => {
    const result = await getIdentitiesContractKeys(
      [identityFixtureA.getId(), identityFixtureB.getId()],
      contractId,
      [KeyPurpose.ENCRYPTION, KeyPurpose.DECRYPTION],
      'contactRequest',
      options,
    );

    const request = mockRequest();

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentitiesContractKeys',
      request,
      options,
    );
    expect(result.getIdentitiesKeys()).to.deep.equal(identitiesContractKeys);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    response.getV0().setProof(proofResponse);

    const result = await getIdentitiesContractKeys(
      [identityFixtureA.getId(), identityFixtureB.getId()],
      contractId,
      [KeyPurpose.ENCRYPTION, KeyPurpose.DECRYPTION],
      'contactRequest',
      options,
    );

    const request = mockRequest();

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentitiesContractKeys',
      request,
      options,
    );
    expect(result.getIdentitiesKeys()).to.deep.equal({});

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

    const request = mockRequest();

    try {
      await getIdentitiesContractKeys(
        [identityFixtureA.getId(), identityFixtureB.getId()],
        contractId,
        [KeyPurpose.ENCRYPTION, KeyPurpose.DECRYPTION],
        'contactRequest',
        options,
      );

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentitiesContractKeys',
        request,
        options,
      );
    }
  });
});
