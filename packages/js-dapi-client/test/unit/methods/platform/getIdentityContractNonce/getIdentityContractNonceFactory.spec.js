const {
  v0: {
    PlatformPromiseClient,
    GetIdentityContractNonceRequest,
    GetIdentityContractNonceResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityContractNonceFactory = require('../../../../../lib/methods/platform/getIdentityContractNonce/getIdentityContractNonceFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getIdentityContractNonceFactory', () => {
  let grpcTransportMock;
  let getIdentityContractNonce;
  let options;
  let response;
  let nonce;
  let identityId;
  let contractId;
  let metadataFixture;
  let proofFixture;
  let proofResponse;

  beforeEach(async function beforeEach() {
    nonce = BigInt(1);
    identityId = Buffer.alloc(32).fill(0);
    contractId = Buffer.alloc(32).fill(1);

    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { GetIdentityContractNonceResponseV0 } = GetIdentityContractNonceResponse;
    response = new GetIdentityContractNonceResponse();
    response.setV0(
      new GetIdentityContractNonceResponseV0()
        .setIdentityContractNonce(nonce)
        .setMetadata(metadata),
    );

    proofResponse = new ProofResponse();

    proofResponse.setQuorumHash(proofFixture.quorumHash);
    proofResponse.setSignature(proofFixture.signature);
    proofResponse.setGrovedbProof(proofFixture.merkleProof);
    proofResponse.setRound(proofFixture.round);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    getIdentityContractNonce = getIdentityContractNonceFactory(grpcTransportMock);

    options = {
      timeout: 1000,
    };
  });

  it('should return identity nonce', async () => {
    const result = await getIdentityContractNonce(identityId, contractId, options);

    const { GetIdentityContractNonceRequestV0 } = GetIdentityContractNonceRequest;
    const request = new GetIdentityContractNonceRequest();
    request.setV0(
      new GetIdentityContractNonceRequestV0()
        .setIdentityId(identityId)
        .setContractId(contractId)
        .setProve(false),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityContractNonce',
      request,
      options,
    );
    expect(result.getIdentityContractNonce()).to.deep.equal(nonce);

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    response.getV0().setIdentityContractNonce(undefined);
    response.getV0().setProof(proofResponse);

    const result = await getIdentityContractNonce(identityId, contractId, options);

    const { GetIdentityContractNonceRequestV0 } = GetIdentityContractNonceRequest;
    const request = new GetIdentityContractNonceRequest();
    request.setV0(
      new GetIdentityContractNonceRequestV0()
        .setIdentityId(identityId)
        .setContractId(contractId)
        .setProve(true),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentityContractNonce',
      request,
      options,
    );

    expect(result.getIdentityContractNonce()).to.deep.equal(BigInt(0));

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(result.getProof()).to.be.an.instanceOf(Proof);
    expect(result.getProof().getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(result.getProof().getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(result.getProof().getSignature()).to.deep.equal(proofFixture.signature);
    expect(result.getProof().getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const { GetIdentityContractNonceRequestV0 } = GetIdentityContractNonceRequest;
    const request = new GetIdentityContractNonceRequest();
    request.setV0(
      new GetIdentityContractNonceRequestV0()
        .setIdentityId(identityId)
        .setContractId(contractId)
        .setProve(false),
    );

    try {
      await getIdentityContractNonce(identityId, contractId, options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentityContractNonce',
        request,
        options,
      );
    }
  });
});
