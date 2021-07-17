const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const {
  v0: {
    GetDataContractResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetDataContractResponseClass = require('../../../../../lib/methods/platform/getDataContract/GetDataContractResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetDataContractResponse', () => {
  let getDataContractResponse;
  let metadataFixture;
  let dataContractFixture;
  let proofFixture;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    dataContractFixture = getDataContractFixture();
    proofFixture = getProofFixture();

    getDataContractResponse = new GetDataContractResponseClass(
      dataContractFixture.toBuffer(),
      new Metadata(metadataFixture),
    );
  });

  it('should return DataContract', () => {
    const dataContract = getDataContractResponse.getDataContract();
    const proof = getDataContractResponse.getProof();

    expect(dataContract).to.deep.equal(dataContractFixture.toBuffer());
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getDataContractResponse = new GetDataContractResponseClass(
      Buffer.alloc(0),
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const dataContract = getDataContractResponse.getDataContract();
    const proof = getDataContractResponse.getProof();

    expect(dataContract).to.deep.equal(Buffer.alloc(0));
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(proof.getStoreTreeProof()).to.deep.equal(proofFixture.storeTreeProof);
    expect(proof.getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    const proto = new GetDataContractResponse();
    proto.setDataContract(dataContractFixture.toBuffer());

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);
    expect(getDataContractResponse).to.be.an.instanceOf(GetDataContractResponseClass);
    expect(getDataContractResponse.getDataContract()).to.deep.equal(dataContractFixture.toBuffer());

    expect(getDataContractResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getDataContractResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getDataContractResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getDataContractResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();
    proofProto.setSignatureLlmqHash(proofFixture.signatureLLMQHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setRootTreeProof(proofFixture.rootTreeProof);
    proofProto.setStoreTreeProof(proofFixture.storeTreeProof);

    const proto = new GetDataContractResponse();

    proto.setDataContract(undefined);
    proto.setProof(proofProto);

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);
    expect(getDataContractResponse).to.be.an.instanceOf(GetDataContractResponseClass);
    expect(getDataContractResponse.getDataContract()).to.deep.equal(Buffer.alloc(0));

    expect(getDataContractResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getDataContractResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getDataContractResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    const proof = getDataContractResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(proof.getStoreTreeProof()).to.deep.equal(proofFixture.storeTreeProof);
    expect(proof.getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    const proto = new GetDataContractResponse();
    proto.setDataContract(dataContractFixture.toBuffer());

    try {
      getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if DataContract is not defined', () => {
    const proto = new GetDataContractResponse();
    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    try {
      getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
