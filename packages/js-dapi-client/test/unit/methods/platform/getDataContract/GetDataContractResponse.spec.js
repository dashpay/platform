const getDataContractFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture');
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

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    dataContractFixture = await getDataContractFixture();
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
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    const { GetDataContractResponseV0 } = GetDataContractResponse;
    const proto = new GetDataContractResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetDataContractResponseV0()
        .setDataContract(dataContractFixture.toBuffer())
        .setMetadata(metadata),
    );

    getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);
    expect(getDataContractResponse).to.be.an.instanceOf(GetDataContractResponseClass);
    expect(getDataContractResponse.getDataContract()).to.deep.equal(dataContractFixture.toBuffer());

    expect(getDataContractResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getDataContractResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getDataContractResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getDataContractResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(getDataContractResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);

    const { GetDataContractResponseV0 } = GetDataContractResponse;
    const proto = new GetDataContractResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetDataContractResponseV0()
        .setDataContract(undefined)
        .setMetadata(metadata)
        .setProof(proofProto),
    );

    getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);
    expect(getDataContractResponse).to.be.an.instanceOf(GetDataContractResponseClass);
    expect(getDataContractResponse.getDataContract()).to.deep.equal(Buffer.alloc(0));

    expect(getDataContractResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getDataContractResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getDataContractResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getDataContractResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    const proof = getDataContractResponse.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    const { GetDataContractResponseV0 } = GetDataContractResponse;
    const proto = new GetDataContractResponse();
    proto.setV0(
      new GetDataContractResponseV0()
        .setDataContract(dataContractFixture.toBuffer()),
    );

    try {
      getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if DataContract is not defined', () => {
    const { GetDataContractResponseV0 } = GetDataContractResponse;
    const proto = new GetDataContractResponse();
    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setV0(
      new GetDataContractResponseV0()
        .setMetadata(metadata),
    );

    try {
      getDataContractResponse = GetDataContractResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
