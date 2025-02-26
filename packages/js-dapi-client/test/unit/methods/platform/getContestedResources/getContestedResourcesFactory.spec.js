const {
  v0: {
    PlatformPromiseClient,
    GetContestedResourcesRequest,
    GetContestedResourcesResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getContestedResourcesFactory = require('../../../../../lib/methods/platform/getContestedResources/getContestedResourcesFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getContestedResourcesFactory', () => {
  let grpcTransportMock;
  let contractIdBuffer;
  let metadataFixture;
  let proofFixture;
  let proofResponse;
  let response;
  let documentTypeName;
  let indexName;
  let startIndexValues;
  let endIndexValues;
  let startAtValueInfo;
  let count;
  let orderAscending;
  let contestedResourceValues;
  let getContestedResources;

  beforeEach(async function beforeEach() {
    contractIdBuffer = Buffer.from('11c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c', 'hex');
    documentTypeName = 'domain';
    indexName = 'normalizedLabel';
    startIndexValues = [];
    endIndexValues = [];
    startAtValueInfo = new GetContestedResourcesRequest
      .GetContestedResourcesRequestV0
      .StartAtValueInfo({
        startValue: (await generateRandomIdentifier()),
        startValueIncluded: true,
      });
    contestedResourceValues = ['EgRkYXNo'];
    count = 1;
    orderAscending = true;

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    getContestedResources = getContestedResourcesFactory(grpcTransportMock);

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { GetContestedResourcesResponseV0 } = GetContestedResourcesResponse;
    response = new GetContestedResourcesResponse();
    response.setV0(
      new GetContestedResourcesResponseV0()
        .setContestedResourceValues(new GetContestedResourcesResponseV0
          .ContestedResourceValues([contestedResourceValues]))
        .setMetadata(metadata),
    );

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    getContestedResources = getContestedResourcesFactory(grpcTransportMock);

    proofResponse = new ProofResponse();
    proofResponse.setQuorumHash(proofFixture.quorumHash);
    proofResponse.setSignature(proofFixture.signature);
    proofResponse.setGrovedbProof(proofFixture.merkleProof);
    proofResponse.setRound(proofFixture.round);
  });

  it('should return contested resources', async () => {
    const options = { prove: false };

    const result = await getContestedResources(
      contractIdBuffer,
      documentTypeName,
      indexName,
      startIndexValues,
      endIndexValues,
      startAtValueInfo,
      count,
      orderAscending,
      options,
    );

    const { GetContestedResourcesRequestV0 } = GetContestedResourcesRequest;

    const request = new GetContestedResourcesRequest();

    request.setV0(
      new GetContestedResourcesRequestV0()
        .setContractId(contractIdBuffer)
        .setDocumentTypeName(documentTypeName)
        .setIndexName(indexName)
        .setStartIndexValuesList([])
        .setEndIndexValuesList([])
        .setStartAtValueInfo(startAtValueInfo)
        .setCount(1)
        .setOrderAscending(true)
        .setProve(!!options.prove),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getContestedResources',
      request,
      options,
    );
    expect(result.getContestedResources()).to.equal(contestedResourceValues);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    const options = { prove: true };
    response.getV0().setProof(proofResponse);

    const result = await getContestedResources(
      contractIdBuffer,
      documentTypeName,
      indexName,
      startIndexValues,
      endIndexValues,
      startAtValueInfo,
      count,
      orderAscending,
      options,
    );

    const { GetContestedResourcesRequestV0 } = GetContestedResourcesRequest;
    const request = new GetContestedResourcesRequest();

    request.setV0(
      new GetContestedResourcesRequestV0()
        .setContractId(contractIdBuffer)
        .setDocumentTypeName(documentTypeName)
        .setIndexName(indexName)
        .setStartIndexValuesList([])
        .setEndIndexValuesList([])
        .setStartAtValueInfo(startAtValueInfo)
        .setCount(1)
        .setOrderAscending(true)
        .setProve(!!options.prove),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getContestedResources',
      request,
      options,
    );

    expect(result.getContestedResources()).to.equal('');
    expect(result.getMetadata()).to.deep.equal(metadataFixture);

    expect(result.getProof()).to.be.an.instanceOf(Proof);
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
});
