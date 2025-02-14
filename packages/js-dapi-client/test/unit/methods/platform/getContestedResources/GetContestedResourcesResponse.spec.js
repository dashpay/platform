const {
  v0: {
    GetContestedResourcesResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetContestedResourceResponseClass = require('../../../../../lib/methods/platform/getContestedResources/getContestedResourcesResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetContestedResourcesResponse', () => {
  let getContestedResourcesResponse;
  let metadataFixture;
  let proto;
  let proofFixture;
  let contestedResourceValues;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();
    contestedResourceValues = ['EgRkYXNo'];

    const { GetContestedResourcesResponseV0 } = GetContestedResourcesResponse;
    proto = new GetContestedResourcesResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setV0(
      new GetContestedResourcesResponseV0()
        .setContestedResourceValues(new GetContestedResourcesResponseV0
          .ContestedResourceValues([contestedResourceValues]))
        .setMetadata(metadata),
    );

    getContestedResourcesResponse = new GetContestedResourceResponseClass(
      contestedResourceValues,
      1,
      new Metadata(metadataFixture),
    );
  });

  it('should return contested resources', () => {
    const contestedResources = getContestedResourcesResponse.getContestedResources();
    const proof = getContestedResourcesResponse.getProof();

    expect(contestedResources).to.deep.equal(contestedResourceValues);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', async () => {
    getContestedResourcesResponse = new GetContestedResourceResponseClass(
      '',
      1,
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );
    const contestedResources = getContestedResourcesResponse.getContestedResources();
    const proof = getContestedResourcesResponse.getProof();

    expect(contestedResources).to.equal('');

    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    getContestedResourcesResponse = GetContestedResourceResponseClass.createFromProto(proto);
    expect(getContestedResourcesResponse).to.be.an.instanceOf(GetContestedResourceResponseClass);
    expect(getContestedResourcesResponse.getContestedResources())
      .to.deep.equal(contestedResourceValues);

    expect(getContestedResourcesResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getContestedResourcesResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getContestedResourcesResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getContestedResourcesResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);

    proto.getV0().setProof(proofProto);

    getContestedResourcesResponse = GetContestedResourceResponseClass.createFromProto(proto);

    expect(getContestedResourcesResponse).to.be.an.instanceOf(GetContestedResourceResponseClass);
    expect(getContestedResourcesResponse.getContestedResources()).to.equal('');

    expect(getContestedResourcesResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getContestedResourcesResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getContestedResourcesResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    const proof = getContestedResourcesResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getContestedResourcesResponse = GetContestedResourceResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
