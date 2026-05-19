import getDocumentsFixture from '@dashevo/wasm-dpp/lib/test/fixtures/getDocumentsFixture.js';
import dapiGrpc from '@dashevo/dapi-grpc';
import GetDocumentsResponseClass from '../../../../../lib/methods/platform/getDocuments/GetDocumentsResponse.js';
import getMetadataFixture from '../../../../../lib/test/fixtures/getMetadataFixture.js';
import InvalidResponseError from '../../../../../lib/methods/platform/response/errors/InvalidResponseError.js';
import getProofFixture from '../../../../../lib/test/fixtures/getProofFixture.js';
import Proof from '../../../../../lib/methods/platform/response/Proof.js';
import Metadata from '../../../../../lib/methods/platform/response/Metadata.js';

const {
  v0: {
    GetDocumentsResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = dapiGrpc;

describe('GetDocumentsResponse', () => {
  let getDocumentsResponse;
  let metadataFixture;
  let documentsFixture;
  let proto;
  let serializedDocuments;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    documentsFixture = await getDocumentsFixture();
    proofFixture = getProofFixture();

    serializedDocuments = documentsFixture
      .map((document) => new TextEncoder().encode(JSON.stringify(document)));

    const { GetDocumentsResponseV0 } = GetDocumentsResponse;
    proto = new GetDocumentsResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetDocumentsResponseV0()
        .setDocuments(
          new GetDocumentsResponseV0.Documents()
            .setDocumentsList(serializedDocuments),
        ).setMetadata(metadata),
    );

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
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    getDocumentsResponse = GetDocumentsResponseClass.createFromProto(proto);
    expect(getDocumentsResponse).to.be.an.instanceOf(GetDocumentsResponseClass);
    expect(getDocumentsResponse.getDocuments()).to.deep.equal(serializedDocuments);

    expect(getDocumentsResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getDocumentsResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getDocumentsResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getDocumentsResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(getDocumentsResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);

    proto.getV0().setProof(proofProto);

    getDocumentsResponse = GetDocumentsResponseClass.createFromProto(proto);

    expect(getDocumentsResponse).to.be.an.instanceOf(GetDocumentsResponseClass);
    expect(getDocumentsResponse.getDocuments()).to.deep.members([]);

    expect(getDocumentsResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getDocumentsResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getDocumentsResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getDocumentsResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    const proof = getDocumentsResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getDocumentsResponse = GetDocumentsResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
