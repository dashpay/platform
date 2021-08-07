const cbor = require('cbor');
const Identifier = require('@dashevo/dpp/lib/Identifier');

const {
  v0: {
    PlatformPromiseClient,
    GetDocumentsRequest,
    GetDocumentsResponse,
    ResponseMetadata,
    Proof: ProofResponse,
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const getDocumentsFactory = require('../../../../../lib/methods/platform/getDocuments/getDocumentsFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getDocumentsFactory', () => {
  let grpcTransportMock;
  let getDocuments;
  let options;
  let contractIdBuffer;
  let contractIdIdentifier;
  let type;
  let documentsFixture;
  let serializedDocuments;
  let metadataFixture;
  let proofFixture;
  let proofResponse;
  let response;
  let storeTreeProofsProto;

  beforeEach(function beforeEach() {
    type = 'niceDocument';
    contractIdBuffer = Buffer.from('11c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c', 'hex');
    contractIdIdentifier = Identifier.from(contractIdBuffer);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    options = {
      limit: 10,
      orderBy: [
        ['order', 'asc'],
      ],
      startAt: 1,
      where: [['lastName', '==', 'unknown']],
      startAfter: 10,
    };

    documentsFixture = getDocumentsFixture();
    serializedDocuments = documentsFixture
      .map((document) => Buffer.from(JSON.stringify(document)));

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    response = new GetDocumentsResponse();
    response.setDocumentsList(serializedDocuments);
    response.setMetadata(metadata);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    getDocuments = getDocumentsFactory(grpcTransportMock);

    proofResponse = new ProofResponse();
    storeTreeProofsProto = new StoreTreeProofs();
    storeTreeProofsProto.setIdentitiesProof(proofFixture.storeTreeProofs.identitiesProof);
    storeTreeProofsProto.setPublicKeyHashesToIdentityIdsProof(
      proofFixture.storeTreeProofs.publicKeyHashesToIdentityIdsProof,
    );
    storeTreeProofsProto.setDataContractsProof(proofFixture.storeTreeProofs.dataContractsProof);
    storeTreeProofsProto.setDocumentsProof(proofFixture.storeTreeProofs.documentsProof);
    proofResponse.setSignatureLlmqHash(proofFixture.signatureLLMQHash);
    proofResponse.setSignature(proofFixture.signature);
    proofResponse.setRootTreeProof(proofFixture.rootTreeProof);
    proofResponse.setStoreTreeProofs(storeTreeProofsProto);
  });

  it('should return documents when contract id is buffer', async () => {
    const result = await getDocuments(contractIdBuffer, type, options);

    const request = new GetDocumentsRequest();
    request.setDataContractId(contractIdBuffer);
    request.setDocumentType(type);
    request.setLimit(options.limit);
    request.setWhere(cbor.encode(options.where));
    request.setOrderBy(cbor.encode(options.orderBy));
    request.setStartAfter(options.startAfter);
    request.setStartAt(options.startAt);
    request.setProve(false);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getDocuments',
      request,
      options,
    );
    expect(result.getDocuments()).to.deep.equal(serializedDocuments);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    response.setDocumentsList([]);
    response.setProof(proofResponse);

    const result = await getDocuments(contractIdBuffer, type, options);

    const request = new GetDocumentsRequest();
    request.setDataContractId(contractIdBuffer);
    request.setDocumentType(type);
    request.setLimit(options.limit);
    request.setWhere(cbor.encode(options.where));
    request.setOrderBy(cbor.encode(options.orderBy));
    request.setStartAfter(options.startAfter);
    request.setStartAt(options.startAt);
    request.setProve(true);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getDocuments',
      request,
      options,
    );

    expect(result.getDocuments()).to.deep.members([]);

    expect(result.getMetadata()).to.deep.equal(metadataFixture);

    expect(result.getProof()).to.be.an.instanceOf(Proof);
    expect(result.getProof().getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(result.getProof().getStoreTreeProofs()).to.deep.equal(proofFixture.storeTreeProofs);
    expect(result.getProof().getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(result.getProof().getSignature()).to.deep.equal(proofFixture.signature);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getMetadata().getHeight()).to.equal(metadataFixture.height);
    expect(result.getMetadata().getCoreChainLockedHeight()).to.equal(
      metadataFixture.coreChainLockedHeight,
    );
  });

  it('should return documents when contract id is identifier', async () => {
    const result = await getDocuments(contractIdIdentifier, type, options);

    const request = new GetDocumentsRequest();
    request.setDataContractId(contractIdBuffer);
    request.setDocumentType(type);
    request.setLimit(options.limit);
    request.setWhere(cbor.encode(options.where));
    request.setOrderBy(cbor.encode(options.orderBy));
    request.setStartAfter(options.startAfter);
    request.setStartAt(options.startAt);
    request.setProve(false);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getDocuments',
      request,
      options,
    );
    expect(result.getDocuments()).to.deep.equal(serializedDocuments);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });
});
