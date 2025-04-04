const {
  v0: {
    PlatformPromiseClient,
    WaitForStateTransitionResultRequest,
    StateTransitionBroadcastError,
    WaitForStateTransitionResultResponse,
    Proof,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');
const cbor = require('cbor');

const waitForStateTransitionResultFactory = require('../../../../../lib/methods/platform/waitForStateTransitionResult/waitForStateTransitionResultFactory');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');

describe('waitForStateTransitionResultFactory', () => {
  let grpcTransportMock;
  let options;
  let response;
  let hash;
  let waitForStateTransitionResult;
  let metadataFixture;

  beforeEach(function beforeEach() {
    hash = Buffer.from('hash');
    metadataFixture = getMetadataFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    response = new WaitForStateTransitionResultResponse();
    response.setV0(
      new WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0()
        .setMetadata(metadata),
    );

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
      throwDeadlineExceeded: true,
      retry: 0,
    };

    waitForStateTransitionResult = waitForStateTransitionResultFactory(grpcTransportMock);
  });

  it('should return response', async () => {
    options.prove = false;

    const result = await waitForStateTransitionResult(hash, options);

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(result.getError()).to.equal(undefined);
    expect(result.getProof()).to.equal(undefined);

    const { WaitForStateTransitionResultRequestV0 } = WaitForStateTransitionResultRequest;
    const request = new WaitForStateTransitionResultRequest();
    request.setV0(
      new WaitForStateTransitionResultRequestV0()
        .setStateTransitionHash(hash)
        .setProve(false),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'waitForStateTransitionResult',
      request,
      options,
    );
  });

  it('should return response with proof', async () => {
    const proof = new Proof();

    proof.setGrovedbProof(Buffer.from('merkleProof'));
    proof.setQuorumHash(Buffer.from('quorumHash'));
    proof.setSignature(Buffer.from('signature'));
    proof.setRound(42);

    response.getV0().setProof(proof);

    options.prove = true;

    const result = await waitForStateTransitionResult(hash, options);

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(result.getError()).to.equal(undefined);
    expect(result.getProof()).to.be.deep.equal({
      merkleProof: Buffer.from('merkleProof'),
      quorumHash: Buffer.from('quorumHash'),
      signature: Buffer.from('signature'),
      round: 42,
    });
    expect(result.getProof().getSignature()).to.deep.equal(Buffer.from('signature'));
    expect(result.getProof().getGrovedbProof()).to.deep.equal(Buffer.from('merkleProof'));
    expect(result.getProof().getQuorumHash()).to.deep.equal(Buffer.from('quorumHash'));
    expect(result.getProof().getRound()).to.deep.equal(42);

    const { WaitForStateTransitionResultRequestV0 } = WaitForStateTransitionResultRequest;
    const request = new WaitForStateTransitionResultRequest();
    request.setV0(
      new WaitForStateTransitionResultRequestV0()
        .setStateTransitionHash(hash)
        .setProve(true),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'waitForStateTransitionResult',
      request,
      options,
    );
  });

  it('should return response with error', async () => {
    const data = cbor.encode({ data: 'error data' });

    const error = new StateTransitionBroadcastError();
    error.setCode(2);
    error.setMessage('Some error');
    error.setData(data);

    response.getV0().setError(error);

    options.prove = true;

    const result = await waitForStateTransitionResult(hash, options);

    expect(result.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(result.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(result.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(result.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(result.getProof()).to.equal(undefined);
    expect(result.getError()).to.be.deep.equal({
      code: 2,
      message: 'Some error',
      data: Buffer.from(data),
    });

    const { WaitForStateTransitionResultRequestV0 } = WaitForStateTransitionResultRequest;
    const request = new WaitForStateTransitionResultRequest();
    request.setV0(
      new WaitForStateTransitionResultRequestV0()
        .setStateTransitionHash(hash)
        .setProve(true),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'waitForStateTransitionResult',
      request,
      options,
    );
  });
});
