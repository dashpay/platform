const { BytesValue } = require('google-protobuf/google/protobuf/wrappers_pb');

const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');
const {
  v0: {
    GetPartialIdentitiesResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetPartialIdentitiesResponseClass = require('../../../../../lib/methods/platform/getPartialIdentities/GetPartialIdentitiesResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetPartialIdentitiesResponse', () => {
  let getIdentitiesResponse;
  let metadataFixture;
  let identityFixture;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    identityFixture = await getIdentityFixture();
    proofFixture = getProofFixture();

    const {
      Identities, IdentityEntry, IdentityValue,
      GetPartialIdentitiesResponseV0,
    } = GetPartialIdentitiesResponse;

    proto = new GetPartialIdentitiesResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetPartialIdentitiesResponseV0().setIdentities(
        new Identities()
          .setIdentityEntriesList([
            new IdentityEntry()
              .setValue(new IdentityValue().setValue(identityFixture.toBuffer())),
          ]),
      ).setMetadata(metadata),
    );

    getIdentitiesResponse = new GetPartialIdentitiesResponseClass(
      [identityFixture.toBuffer()],
      new Metadata(metadataFixture),
    );
  });

  it('should return identities', () => {
    const identities = getIdentitiesResponse.getIdentities();
    const proof = getIdentitiesResponse.getProof();

    expect(identities).to.deep.members([identityFixture.toBuffer()]);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentitiesResponse = new GetPartialIdentitiesResponseClass(
      [],
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identities = getIdentitiesResponse.getIdentities();
    const proof = getIdentitiesResponse.getProof();

    expect(identities).to.deep.members([]);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    getIdentitiesResponse = GetPartialIdentitiesResponseClass.createFromProto(proto);
    expect(getIdentitiesResponse).to.be.an.instanceOf(
      GetPartialIdentitiesResponseClass,
    );
    expect(getIdentitiesResponse.getIdentities()).to.deep.equal([identityFixture.toBuffer()]);

    expect(getIdentitiesResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentitiesResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentitiesResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentitiesResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setProof(proofProto);

    getIdentitiesResponse = GetPartialIdentitiesResponseClass.createFromProto(proto);
    expect(getIdentitiesResponse).to.be.an.instanceOf(
      GetPartialIdentitiesResponseClass,
    );
    expect(getIdentitiesResponse.getIdentities()).to.deep.members([]);
    expect(getIdentitiesResponse.getMetadata()).to.deep.equal(metadataFixture);

    expect(getIdentitiesResponse.getProof())
      .to.be.an.instanceOf(Proof);
    expect(getIdentitiesResponse.getProof().getGrovedbProof())
      .to.deep.equal(proofFixture.merkleProof);
    expect(getIdentitiesResponse.getProof().getQuorumHash())
      .to.deep.equal(proofFixture.quorumHash);
    expect(getIdentitiesResponse.getProof().getSignature())
      .to.deep.equal(proofFixture.signature);
    expect(getIdentitiesResponse.getProof().getRound())
      .to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getIdentitiesResponse = GetPartialIdentitiesResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
