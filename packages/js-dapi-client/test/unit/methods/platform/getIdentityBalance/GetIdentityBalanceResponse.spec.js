const {
  v0: {
    GetIdentityBalanceResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityBalanceResponseClass = require('../../../../../lib/methods/platform/getIdentityBalance/GetIdentityBalanceResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetIdentityBalanceResponse', () => {
  let getIdentityBalanceResponse;
  let metadataFixture;
  let balance;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();
    balance = BigInt(1337);

    const { GetIdentityBalanceResponseV0 } = GetIdentityBalanceResponse;
    proto = new GetIdentityBalanceResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetIdentityBalanceResponseV0()
        .setBalance(balance)
        .setMetadata(metadata),
    );

    getIdentityBalanceResponse = new GetIdentityBalanceResponseClass(
      balance,
      new Metadata(metadataFixture),
    );
  });

  it('should return Identity balance', () => {
    const identityBalance = getIdentityBalanceResponse.getBalance();
    const identityProof = getIdentityBalanceResponse.getProof();

    expect(identityBalance).to.equal(balance);
    expect(identityProof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentityBalanceResponse = new GetIdentityBalanceResponseClass(
      balance,
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identityBalance = getIdentityBalanceResponse.getBalance();
    const proof = getIdentityBalanceResponse.getProof();

    expect(identityBalance).to.equal(balance);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should create an instance from proto', () => {
    getIdentityBalanceResponse = GetIdentityBalanceResponseClass.createFromProto(proto);
    expect(getIdentityBalanceResponse).to.be
      .an.instanceOf(GetIdentityBalanceResponseClass);
    expect(getIdentityBalanceResponse.getBalance()).to.equal(balance);

    expect(getIdentityBalanceResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentityBalanceResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentityBalanceResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentityBalanceResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setBalance(undefined);
    proto.getV0().setProof(proofProto);

    getIdentityBalanceResponse = GetIdentityBalanceResponseClass.createFromProto(proto);

    expect(getIdentityBalanceResponse.getBalance()).to.equal(BigInt(0));

    expect(getIdentityBalanceResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getIdentityBalanceResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getIdentityBalanceResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getIdentityBalanceResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    const proof = getIdentityBalanceResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getIdentityBalanceResponse = GetIdentityBalanceResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
