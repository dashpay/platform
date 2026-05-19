import dapiGrpc from '@dashevo/dapi-grpc';
import GetIdentityKeysResponseClass from '../../../../../lib/methods/platform/getIdentityKeys/GetIdentityKeysResponse.js';
import getMetadataFixture from '../../../../../lib/test/fixtures/getMetadataFixture.js';
import InvalidResponseError from '../../../../../lib/methods/platform/response/errors/InvalidResponseError.js';
import getProofFixture from '../../../../../lib/test/fixtures/getProofFixture.js';
import Proof from '../../../../../lib/methods/platform/response/Proof.js';
import Metadata from '../../../../../lib/methods/platform/response/Metadata.js';

const {
  v0: {
    GetIdentityKeysResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = dapiGrpc;

describe('GetIdentityKeysResponse', () => {
  let getIdentityKeysResponse;
  let metadataFixture;
  let keys;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    keys = [new Uint8Array(41).fill(1), new Uint8Array(48).fill(2), new Uint8Array(55).fill(3)];
    proofFixture = getProofFixture();

    const { GetIdentityKeysResponseV0 } = GetIdentityKeysResponse;
    proto = new GetIdentityKeysResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { Keys } = GetIdentityKeysResponseV0;

    proto.setV0(
      new GetIdentityKeysResponseV0()
        .setKeys(new Keys().setKeysBytesList(keys))
        .setMetadata(metadata),
    );

    getIdentityKeysResponse = new GetIdentityKeysResponseClass(
      keys,
      new Metadata(metadataFixture),
    );
  });

  it('should return Identity Keys', () => {
    const identityKeys = getIdentityKeysResponse.getIdentityKeys();
    const proof = getIdentityKeysResponse.getProof();

    expect(identityKeys).to.deep.equal(keys);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentityKeysResponse = new GetIdentityKeysResponseClass(
      [],
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identityKeys = getIdentityKeysResponse.getIdentityKeys();
    const proof = getIdentityKeysResponse.getProof();

    expect(identityKeys).to.deep.equal([]);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should create an instance from proto', () => {
    getIdentityKeysResponse = GetIdentityKeysResponseClass.createFromProto(proto);
    expect(getIdentityKeysResponse).to.be
      .an.instanceOf(GetIdentityKeysResponseClass);
    expect(getIdentityKeysResponse.getIdentityKeys()).to.deep.equal(keys);

    expect(getIdentityKeysResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentityKeysResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentityKeysResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentityKeysResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setKeys(undefined);
    proto.getV0().setProof(proofProto);

    getIdentityKeysResponse = GetIdentityKeysResponseClass.createFromProto(proto);

    expect(getIdentityKeysResponse.getIdentityKeys())
      .to.deep.equal([]);

    expect(getIdentityKeysResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getIdentityKeysResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getIdentityKeysResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getIdentityKeysResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    const proof = getIdentityKeysResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getIdentityKeysResponse = GetIdentityKeysResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
