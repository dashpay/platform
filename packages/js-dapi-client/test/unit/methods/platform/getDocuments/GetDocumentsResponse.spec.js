const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const {
  v0: {
    GetDocumentsResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetDocumentsResponseClass = require('../../../../../lib/methods/platform/getDocuments/GetDocumentsResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetDocumentsResponse', () => {
  let getDocumentsResponse;
  let metadataFixture;
  let documentsFixture;
  let proto;
  let serializedDocuments;
  let proofFixture;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    documentsFixture = getDocumentsFixture();
    proofFixture = getProofFixture();

    proto = new GetDocumentsResponse();

    serializedDocuments = documentsFixture
      .map((document) => Buffer.from(JSON.stringify(document)));

    proto.setDocumentsList(serializedDocuments);

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getDocumentsResponse = new GetDocumentsResponseClass(
      serializedDocuments,
      new Metadata(metadataFixture),
    );
  });

  it('should return documents', () => {
    const documents = getDocumentsResponse.getDocuments();
    const proof = getDocumentsResponse.getProof();

    expect(documents).to.deep.equal(serializedDocuments);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', async () => {
    getDocumentsResponse = new GetDocumentsResponseClass(
      [],
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const documents = getDocumentsResponse.getDocuments();
    const proof = getDocumentsResponse.getProof();

    expect(documents).to.deep.equal([]);

    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(proof.getStoreTreeProof()).to.deep.equal(proofFixture.storeTreeProof);
    expect(proof.getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    getDocumentsResponse = GetDocumentsResponseClass.createFromProto(proto);
    expect(getDocumentsResponse).to.be.an.instanceOf(GetDocumentsResponseClass);
    expect(getDocumentsResponse.getDocuments()).to.deep.equal(serializedDocuments);

    expect(getDocumentsResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getDocumentsResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getDocumentsResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getDocumentsResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();
    proofProto.setSignatureLlmqHash(proofFixture.signatureLLMQHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setRootTreeProof(proofFixture.rootTreeProof);
    proofProto.setStoreTreeProof(proofFixture.storeTreeProof);

    proto.setDocumentsList([]);
    proto.setProof(proofProto);

    getDocumentsResponse = GetDocumentsResponseClass.createFromProto(proto);

    expect(getDocumentsResponse).to.be.an.instanceOf(GetDocumentsResponseClass);
    expect(getDocumentsResponse.getDocuments()).to.deep.members([]);

    expect(getDocumentsResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getDocumentsResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getDocumentsResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    const proof = getDocumentsResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(proof.getStoreTreeProof()).to.deep.equal(proofFixture.storeTreeProof);
    expect(proof.getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.setMetadata(undefined);

    try {
      getDocumentsResponse = GetDocumentsResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
