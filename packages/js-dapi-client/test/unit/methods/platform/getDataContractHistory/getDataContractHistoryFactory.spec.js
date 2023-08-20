const {
  v0: {
    PlatformPromiseClient,
    GetDataContractHistoryRequest,
    GetDataContractHistoryResponse,
    ResponseMetadata,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const {
  v0: {
    GetDataContractHistoryResponse: {
      DataContractHistory,
      DataContractHistoryEntry,
    },
  },
} = require('@dashevo/dapi-grpc');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const getDataContractHistoryFactory = require('../../../../../lib/methods/platform/getDataContractHistory/getDataContractHistoryFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const ProofClass = require('../../../../../lib/methods/platform/response/Proof');

describe('getDataContractHistoryFactory', () => {
  let grpcTransportMock;
  let getDataContractHistory;
  let options;
  let response;
  let dataContractFixture;
  let dataContractHistoryFixture;
  let metadataFixture;
  let proofFixture;
  let proof;

  beforeEach(function beforeEach() {
    dataContractFixture = getDataContractFixture();
    dataContractHistoryFixture = {
      1000: dataContractFixture.toBuffer(),
      2000: dataContractFixture.toBuffer(),
    };

    const dataContractHistoryEntryProto = new DataContractHistoryEntry();
    dataContractHistoryEntryProto.setDate(1000);
    dataContractHistoryEntryProto.setValue(dataContractFixture.toBuffer());

    const dataContractHistoryEntryProto2 = new DataContractHistoryEntry();
    dataContractHistoryEntryProto2.setDate(2000);
    dataContractHistoryEntryProto2.setValue(dataContractFixture.toBuffer());

    const dataContractHistoryProto = new DataContractHistory();
    dataContractHistoryProto.setDataContractEntriesList([
      dataContractHistoryEntryProto,
      dataContractHistoryEntryProto2,
    ]);

    response = new GetDataContractHistoryResponse();
    response.setDataContractHistory(dataContractHistoryProto);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    response.setMetadata(metadata);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
    };

    getDataContractHistory = getDataContractHistoryFactory(grpcTransportMock);

    proof = new Proof();

    proof.setQuorumHash(proofFixture.quorumHash);
    proof.setSignature(proofFixture.signature);
    proof.setGrovedbProof(proofFixture.merkleProof);
    proof.setRound(proofFixture.round);
  });

  it('should return data contract history', async () => {
    const contractId = dataContractFixture.getId().toBuffer();
    const result = await getDataContractHistory(contractId, 0, 10, 0, options);

    const request = new GetDataContractHistoryRequest();
    request.setId(contractId);
    request.setLimit(10);
    request.setOffset(0);
    request.setStartAtMs(0);
    request.setProve(false);

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getDataContractHistory',
      request,
      options,
    ]);
    expect(result.getDataContractHistory()).to.deep.equal(dataContractHistoryFixture);
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
    response.setDataContractHistory(undefined);

    const contractId = dataContractFixture.getId().toBuffer();
    const result = await getDataContractHistory(contractId, 0, 10, 0, options);

    const request = new GetDataContractHistoryRequest();
    request.setId(contractId);
    request.setLimit(10);
    request.setOffset(0);
    request.setStartAtMs(0);
    request.setProve(true);

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getDataContractHistory',
      request,
      options,
    ]);

    expect(result.getDataContractHistory()).to.deep.equal({});
    expect(result.getProof()).to.be.an.instanceOf(ProofClass);
    expect(result.getProof().getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(result.getProof().getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(result.getProof().getSignature()).to.deep.equal(proofFixture.signature);
    expect(result.getProof().getRound()).to.deep.equal(proofFixture.round);
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

    const request = new GetDataContractHistoryRequest();
    request.setId(contractId.toBuffer());
    request.setLimit(10);
    request.setOffset(0);
    request.setStartAtMs(0);
    request.setProve(false);

    try {
      await getDataContractHistory(contractId, 0, 10, 0, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getDataContractHistory',
        request,
        options,
      );
    }
  });
});
