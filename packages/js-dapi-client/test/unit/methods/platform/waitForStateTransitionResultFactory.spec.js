const {
  v0: {
    PlatformPromiseClient,
    WaitForStateTransitionResultRequest,
    StateTransitionBroadcastError,
    WaitForStateTransitionResultResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');
const cbor = require('cbor');

const waitForStateTransitionResultFactory = require('../../../../lib/methods/platform/waitForStateTransitionResultFactory');

describe('waitForStateTransitionResultFactory', () => {
  let grpcTransportMock;
  let options;
  let response;
  let hash;
  let waitForStateTransitionResult;

  beforeEach(function beforeEach() {
    hash = Buffer.from('hash');
    response = new WaitForStateTransitionResultResponse();

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

    expect(result).to.be.deep.equal({});

    const request = new WaitForStateTransitionResultRequest();
    request.setStateTransitionHash(hash);
    request.setProve(false);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'waitForStateTransitionResult',
      request,
      options,
    );
  });

  it('should return response with proof', async () => {
    const proof = new Proof();
    proof.setRootTreeProof(Buffer.from('rootTreeProof'));
    proof.setStoreTreeProof(Buffer.from('storeTreeProof'));

    response.setProof(proof);

    options.prove = true;

    const result = await waitForStateTransitionResult(hash, options);

    expect(result).to.be.deep.equal({
      proof: {
        rootTreeProof: Buffer.from('rootTreeProof'),
        storeTreeProof: Buffer.from('storeTreeProof'),
      },
    });

    const request = new WaitForStateTransitionResultRequest();
    request.setStateTransitionHash(hash);
    request.setProve(true);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'waitForStateTransitionResult',
      request,
      options,
    );
  });

  it('should return response with error', async () => {
    const error = new StateTransitionBroadcastError();
    error.setCode(2);
    error.setMessage('Some error');
    error.setData(cbor.encode({ data: 'error data' }));

    response.setError(error);

    options.prove = true;

    const result = await waitForStateTransitionResult(hash, options);

    expect(result).to.be.deep.equal({
      error: {
        code: 2,
        message: 'Some error',
        data: { data: 'error data' },
      },
    });

    const request = new WaitForStateTransitionResultRequest();
    request.setStateTransitionHash(hash);
    request.setProve(true);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'waitForStateTransitionResult',
      request,
      options,
    );
  });
});
