import dapiGrpc from '@dashevo/dapi-grpc';
import cbor from 'cbor';

import waitForStateTransitionResultFactory from '../../../../../lib/methods/platform/waitForStateTransitionResult/waitForStateTransitionResultFactory.js';
import getMetadataFixture from '../../../../../lib/test/fixtures/getMetadataFixture.js';

const {
  v0: {
    PlatformPromiseClient,
    WaitForStateTransitionResultRequest,
    StateTransitionBroadcastError,
    WaitForStateTransitionResultResponse,
    Proof,
    ResponseMetadata,
  },
} = dapiGrpc;

const encoder = new TextEncoder();

describe('waitForStateTransitionResultFactory', () => {
  let grpcTransportMock;
  let options;
  let response;
  let hash;
  let waitForStateTransitionResult;
  let metadataFixture;

  beforeEach(function beforeEach() {
    hash = encoder.encode('hash');
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

    proof.setGrovedbProof(encoder.encode('merkleProof'));
    proof.setQuorumHash(encoder.encode('quorumHash'));
    proof.setSignature(encoder.encode('signature'));
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
      merkleProof: encoder.encode('merkleProof'),
      quorumHash: encoder.encode('quorumHash'),
      signature: encoder.encode('signature'),
      round: 42,
    });
    expect(result.getProof().getSignature()).to.deep.equal(encoder.encode('signature'));
    expect(result.getProof().getGrovedbProof()).to.deep.equal(encoder.encode('merkleProof'));
    expect(result.getProof().getQuorumHash()).to.deep.equal(encoder.encode('quorumHash'));
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
      data: new Uint8Array(data),
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
