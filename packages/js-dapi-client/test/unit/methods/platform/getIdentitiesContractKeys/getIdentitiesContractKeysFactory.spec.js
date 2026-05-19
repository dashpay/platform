import dapiGrpc from '@dashevo/dapi-grpc';
import generateRandomIdentifier from '@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync.js';
import getIdentityFixture from '@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture.js';
import getMetadataFixture from '../../../../../lib/test/fixtures/getMetadataFixture.js';
import getProofFixture from '../../../../../lib/test/fixtures/getProofFixture.js';
import getIdentitiesContractKeysFactory from '../../../../../lib/methods/platform/getIdentitiesContractKeys/getIdentitiesContractKeysFactory.js';
import Proof from '../../../../../lib/methods/platform/response/Proof.js';

const {
  v0: {
    PlatformPromiseClient,
    GetIdentitiesContractKeysRequest,
    GetIdentitiesContractKeysResponse,
    KeyPurpose,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = dapiGrpc;

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
              .setIdentityId(identityFixtureA.getId().toBytes())
              .setKeysList([
                new PurposeKeys()
                  .setPurpose(KeyPurpose.ENCRYPTION)
                  .setKeysBytesList(identityFixtureA.getPublicKeys()
                    .map((key) => new Uint8Array(key.toBuffer()))),
              ]),
            new IdentityKeys()
              .setIdentityId(identityFixtureB.getId().toBytes())
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
            [new Uint8Array(identityFixtureA.getId()), new Uint8Array(identityFixtureB.getId())],
          )
          .setContractId(new Uint8Array(contractId))
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
