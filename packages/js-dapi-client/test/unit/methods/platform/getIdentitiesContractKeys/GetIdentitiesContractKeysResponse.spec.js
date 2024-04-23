const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');
const {
  v0: {
    GetIdentitiesContractKeysResponse,
    ResponseMetadata,
    KeyPurpose,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentitiesContractKeysResponseClass = require('../../../../../lib/methods/platform/getIdentitiesContractKeys/GetIdentitiesContractKeysResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetIdentitiesContractKeysResponse', () => {
  let getIdentitiesContractKeysResponse;
  let metadataFixture;
  let identityFixtureA;
  let identityFixtureB;
  let identitiesContractKeys;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    identityFixtureA = await getIdentityFixture(await generateRandomIdentifier());
    identityFixtureB = await getIdentityFixture(await generateRandomIdentifier());
    proofFixture = getProofFixture();

    const {
      GetIdentitiesContractKeysResponseV0,
    } = GetIdentitiesContractKeysResponse;

    const { IdentitiesKeys, IdentityKeys, PurposeKeys } = GetIdentitiesContractKeysResponseV0;

    proto = new GetIdentitiesContractKeysResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
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

    getIdentitiesContractKeysResponse = new GetIdentitiesContractKeysResponseClass(
      identitiesContractKeys,
      new Metadata(metadataFixture),
    );
  });

  it('should return identities keys', () => {
    const keys = getIdentitiesContractKeysResponse.getIdentitiesKeys();
    const proof = getIdentitiesContractKeysResponse.getProof();

    expect(keys).to.deep.equal(identitiesContractKeys);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentitiesContractKeysResponse = new GetIdentitiesContractKeysResponseClass(
      {},
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const keys = getIdentitiesContractKeysResponse.getIdentitiesKeys();
    const proof = getIdentitiesContractKeysResponse.getProof();

    expect(keys).to.deep.equal({});
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    getIdentitiesContractKeysResponse = GetIdentitiesContractKeysResponseClass
      .createFromProto(proto);
    expect(getIdentitiesContractKeysResponse).to.be.an.instanceOf(
      GetIdentitiesContractKeysResponseClass,
    );

    expect(getIdentitiesContractKeysResponse.getIdentitiesKeys()).to.deep.equal(
      identitiesContractKeys,
    );

    expect(getIdentitiesContractKeysResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentitiesContractKeysResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentitiesContractKeysResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentitiesContractKeysResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setProof(proofProto);

    getIdentitiesContractKeysResponse = GetIdentitiesContractKeysResponseClass
      .createFromProto(proto);
    expect(getIdentitiesContractKeysResponse).to.be.an.instanceOf(
      GetIdentitiesContractKeysResponseClass,
    );
    expect(getIdentitiesContractKeysResponse.getIdentitiesKeys()).to.deep.equal({});
    expect(getIdentitiesContractKeysResponse.getMetadata()).to.deep.equal(metadataFixture);

    expect(getIdentitiesContractKeysResponse.getProof())
      .to.be.an.instanceOf(Proof);
    expect(getIdentitiesContractKeysResponse.getProof().getGrovedbProof())
      .to.deep.equal(proofFixture.merkleProof);
    expect(getIdentitiesContractKeysResponse.getProof().getQuorumHash())
      .to.deep.equal(proofFixture.quorumHash);
    expect(getIdentitiesContractKeysResponse.getProof().getSignature())
      .to.deep.equal(proofFixture.signature);
    expect(getIdentitiesContractKeysResponse.getProof().getRound())
      .to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getIdentitiesContractKeysResponse = GetIdentitiesContractKeysResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
