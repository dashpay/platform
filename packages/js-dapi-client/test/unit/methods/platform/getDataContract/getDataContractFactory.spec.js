const {
  v0: {
    PlatformPromiseClient,
    GetDataContractRequest,
    GetDataContractResponse,
    ResponseMetadata,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const getDataContractFactory = require('../../../../../lib/methods/platform/getDataContract/getDataContractFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const ProofClass = require('../../../../../lib/methods/platform/response/Proof');

describe('getDataContractFactory', () => {
  let grpcTransportMock;
  let getDataContract;
  let options;
  let response;
  let dataContractFixture;
  let metadataFixture;
  let proofFixture;
  let proof;

  beforeEach(function beforeEach() {
    dataContractFixture = getDataContractFixture();

    response = new GetDataContractResponse();
    response.setDataContract(dataContractFixture.toBuffer());

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    response.setMetadata(metadata);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
    };

    getDataContract = getDataContractFactory(grpcTransportMock);

    proof = new Proof();

    proof.setSignatureLlmqHash(proofFixture.signatureLLMQHash);
    proof.setSignature(proofFixture.signature);
    proof.setMerkleProof(proofFixture.merkleProof);
  });

  it('should return data contract', async () => {
    const contractId = dataContractFixture.getId();
    const result = await getDataContract(contractId, options);

    const request = new GetDataContractRequest();
    request.setId(contractId);
    request.setProve(false);

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getDataContract',
      request,
      options,
    ]);
    expect(result.getDataContract()).to.deep.equal(dataContractFixture.toBuffer());
    expect(result.getProof()).to.equal(undefined);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getMetadata().getHeight()).to.equal(metadataFixture.height);
    expect(result.getMetadata().getCoreChainLockedHeight()).to.equal(
      metadataFixture.coreChainLockedHeight,
    );
  });

  it('should return proof', async () => {
    options.prove = true;
    response.setProof(proof);
    response.setDataContract(undefined);

    const contractId = dataContractFixture.getId();
    const result = await getDataContract(contractId, options);

    const request = new GetDataContractRequest();
    request.setId(contractId);
    request.setProve(true);

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getDataContract',
      request,
      options,
    ]);

    expect(result.getDataContract()).to.deep.equal(Buffer.alloc(0));
    expect(result.getProof()).to.be.an.instanceOf(ProofClass);
    expect(result.getProof().getMerkleProof()).to.deep.equal(proofFixture.merkleProof);
    expect(result.getProof().getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(result.getProof().getSignature()).to.deep.equal(proofFixture.signature);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getMetadata().getHeight()).to.equal(metadataFixture.height);
    expect(result.getMetadata().getCoreChainLockedHeight()).to.equal(
      metadataFixture.coreChainLockedHeight,
    );
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');
    const contractId = dataContractFixture.getId();

    grpcTransportMock.request.throws(error);

    const request = new GetDataContractRequest();
    request.setId(contractId.toBuffer());
    request.setProve(false);

    try {
      await getDataContract(contractId, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getDataContract',
        request,
        options,
      );
    }
  });
});
