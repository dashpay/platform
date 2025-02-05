const { UInt32Value } = require('google-protobuf/google/protobuf/wrappers_pb');

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
      GetDataContractHistoryResponseV0: {
        DataContractHistory,
        DataContractHistoryEntry,
      },
    },
  },
} = require('@dashevo/dapi-grpc');

const getDataContractFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture');

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

  beforeEach(async function beforeEach() {
    dataContractFixture = await getDataContractFixture();

    dataContractHistoryFixture = [{
      date: BigInt(10000),
      value: dataContractFixture.toBuffer(),
    },
    {
      date: BigInt(20000),
      value: dataContractFixture.toBuffer(),
    }];

    const dataContractHistoryEntryProto = new DataContractHistoryEntry();
    dataContractHistoryEntryProto.setDate('10000');
    dataContractHistoryEntryProto.setValue(dataContractFixture.toBuffer());

    const dataContractHistoryEntryProto2 = new DataContractHistoryEntry();
    dataContractHistoryEntryProto2.setDate('20000');
    dataContractHistoryEntryProto2.setValue(dataContractFixture.toBuffer());

    const dataContractHistoryProto = new DataContractHistory();
    dataContractHistoryProto.setDataContractEntriesList([
      dataContractHistoryEntryProto,
      dataContractHistoryEntryProto2,
    ]);

    const { GetDataContractHistoryResponseV0 } = GetDataContractHistoryResponse;
    response = new GetDataContractHistoryResponse();

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    response.setV0(
      new GetDataContractHistoryResponseV0()
        .setDataContractHistory(dataContractHistoryProto)
        .setMetadata(metadata),
    );

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
    const result = await getDataContractHistory(contractId, BigInt(0), 10, 0, options);

    const { GetDataContractHistoryRequestV0 } = GetDataContractHistoryRequest;
    const request = new GetDataContractHistoryRequest();
    request.setV0(
      new GetDataContractHistoryRequestV0()
        .setId(contractId)
        .setLimit(new UInt32Value([10]))
        .setOffset(new UInt32Value([0]))
        .setStartAtMs('0')
        .setProve(false),
    );

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getDataContractHistory',
      request,
      options,
    ]);
    expect(result.getDataContractHistory()).to.deep.equal(dataContractHistoryFixture);

    expect(result.getProof()).to.equal(undefined);

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);
  });

  it('should return proof', async () => {
    options.prove = true;
    response.getV0().setProof(proof);
    response.getV0().setDataContractHistory(undefined);

    const contractId = dataContractFixture.getId().toBuffer();
    const result = await getDataContractHistory(contractId, BigInt(0), 10, 0, options);

    const { GetDataContractHistoryRequestV0 } = GetDataContractHistoryRequest;
    const request = new GetDataContractHistoryRequest();
    request.setV0(
      new GetDataContractHistoryRequestV0()
        .setId(contractId)
        .setLimit(new UInt32Value([10]))
        .setOffset(new UInt32Value([0]))
        .setStartAtMs('0')
        .setProve(true),
    );

    expect(grpcTransportMock.request.getCall(0).args).to.have.deep.members([
      PlatformPromiseClient,
      'getDataContractHistory',
      request,
      options,
    ]);

    expect(result.getDataContractHistory()).to.deep.equal(null);

    expect(result.getProof()).to.be.an.instanceOf(ProofClass);
    expect(result.getProof().getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(result.getProof().getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(result.getProof().getSignature()).to.deep.equal(proofFixture.signature);
    expect(result.getProof().getRound()).to.deep.equal(proofFixture.round);

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');
    const contractId = dataContractFixture.getId();

    grpcTransportMock.request.throws(error);

    const { GetDataContractHistoryRequestV0 } = GetDataContractHistoryRequest;
    const request = new GetDataContractHistoryRequest();
    request.setV0(
      new GetDataContractHistoryRequestV0()
        .setId(contractId.toBuffer())
        .setLimit(new UInt32Value([10]))
        .setOffset(new UInt32Value([0]))
        .setStartAtMs('0')
        .setProve(false),
    );

    try {
      await getDataContractHistory(contractId, BigInt(0), 10, 0, options);

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
