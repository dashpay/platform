const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      DeadlineExceededGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    WaitForStateTransitionResultResponse,
    WaitForStateTransitionResultRequest,
    GetProofsResponse,
    GetProofsRequest,
    StateTransitionBroadcastError,
    Proof,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');
const getIdentityCreateTransitionFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const { default: loadWasmDpp, DashPlatformProtocol } = require('@dashevo/wasm-dpp');

const { EventEmitter } = require('events');

const cbor = require('cbor');
const NotFoundGrpcError = require('@dashevo/grpc-common/lib/server/error/NotFoundGrpcError');
const UnavailableGrpcError = require('@dashevo/grpc-common/lib/server/error/UnavailableGrpcError');

const BlockchainListener = require('../../../../../lib/externalApis/tenderdash/BlockchainListener');
const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');
const fetchProofForStateTransitionFactory = require('../../../../../lib/externalApis/drive/fetchProofForStateTransitionFactory');
const waitForTransactionToBeProvableFactory = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionToBeProvableFactory');

const waitForTransactionResult = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionResult');
const waitForStateTransitionResultHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/waitForStateTransitionResultHandlerFactory');

describe('waitForStateTransitionResultHandlerFactory', () => {
  let call;
  let waitForStateTransitionResultHandler;
  let driveClientMock;
  let tenderDashWsClientMock;
  let blockchainListener;
  let hash;
  let proofFixture;
  let wsMessagesFixture;
  let stateTransitionFixture;
  let request;
  let requestPayloadMock;
  let fetchProofForStateTransition;
  let waitForTransactionToBeProvable;
  let transactionNotFoundError;
  let createGrpcErrorFromDriveResponseMock;
  let errorInfo;

  before(async () => {
    await loadWasmDpp();
  });

  beforeEach(async function beforeEach() {
    const hashString = '56458F2D8A8617EA322931B72C103CDD93820004E534295183A6EF215B93C76E';
    hash = Buffer.from(hashString, 'hex');

    errorInfo = {
      message: 'Identity not found',
      metadata: {
        error: 'some data',
      },
    };

    stateTransitionFixture = await getIdentityCreateTransitionFixture();
    const stateTransitionBase64 = stateTransitionFixture.toBuffer().toString('base64');

    wsMessagesFixture = {
      success: {
        query: "tm.event = 'Tx'",
        data: {
          type: 'tendermint/event/Tx',
          value: {
            height: '145',
            tx: stateTransitionBase64,
            result: {},
          },
        },
        events: [
          {
            type: 'tm',
            attributes: [
              {
                key: 'event',
                value: 'Tx',
                index: false,
              },
            ],
          },
          {
            type: 'tx',
            attributes: [
              {
                key: 'hash',
                value: '56458F2D8A8617EA322931B72C103CDD93820004E534295183A6EF215B93C76E',
                index: false,
              },
            ],
          },
          {
            type: 'tx',
            attributes: [
              {
                key: 'height',
                value: '145',
                index: false,
              },
            ],
          },
        ],
      },
      error: {
        query: "tm.event = 'Tx'",
        data: {
          type: 'tendermint/event/Tx',
          value: {
            height: '135',
            tx: stateTransitionBase64,
            result: {
              code: 1043,
              info: cbor.encode(errorInfo).toString('base64'),
            },
          },
        },
        events:
          [
            {
              type: 'tm',
              attributes: [
                {
                  key: 'event',
                  value: 'Tx',
                  index: false,
                },
              ],
            },
            {
              type: 'tx',
              attributes: [
                {
                  key: 'hash',
                  value: '56458F2D8A8617EA322931B72C103CDD93820004E534295183A6EF215B93C76E',
                  index: false,
                },
              ],
            },
            {
              type: 'tx',
              attributes: [
                {
                  key: 'height',
                  value: '135',
                  index: false,
                },
              ],
            },
          ],
      },
    };

    requestPayloadMock = {
      getStateTransitionHash: this.sinon.stub().returns(hash),
      getProve: this.sinon.stub().returns(false),
    };
    call = new GrpcCallMock(this.sinon, {
      getV0: () => requestPayloadMock,
    });

    tenderDashWsClientMock = new EventEmitter();
    tenderDashWsClientMock.subscribe = this.sinon.stub();
    tenderDashWsClientMock.isConnected = true;

    const dpp = new DashPlatformProtocol(null, 1);

    proofFixture = new Proof();
    proofFixture.setGrovedbProof(Buffer.alloc(1, 1));
    proofFixture.setRound(42);

    const metadataFixture = new ResponseMetadata();
    metadataFixture.setHeight(42);
    metadataFixture.setCoreChainLockedHeight(41);
    metadataFixture.setTimeMs(new Date().getTime());
    metadataFixture.setProtocolVersion(1);

    const response = new GetProofsResponse();
    response.setV0(
      new GetProofsResponse.GetProofsResponseV0()
        .setProof(proofFixture)
        .setMetadata(metadataFixture),
    );

    driveClientMock = {
      getProofs: this.sinon.stub().resolves(response),
    };

    blockchainListener = new BlockchainListener(tenderDashWsClientMock);
    blockchainListener.start();

    fetchProofForStateTransition = fetchProofForStateTransitionFactory(driveClientMock);

    transactionNotFoundError = new Error();

    transactionNotFoundError.code = -32603;
    transactionNotFoundError.data = `tx (${hashString}) not found, err: %!w(<nil>)`;

    const getExistingTransactionResult = this.sinon.stub().rejects(transactionNotFoundError);

    waitForTransactionToBeProvable = waitForTransactionToBeProvableFactory(
      waitForTransactionResult,
      getExistingTransactionResult,
    );

    createGrpcErrorFromDriveResponseMock = this.sinon.stub().returns(
      new NotFoundGrpcError(errorInfo.message, errorInfo.metadata),
    );

    waitForStateTransitionResultHandler = waitForStateTransitionResultHandlerFactory(
      fetchProofForStateTransition,
      waitForTransactionToBeProvable,
      blockchainListener,
      dpp,
      createGrpcErrorFromDriveResponseMock,
      1000,
    );
  });

  it('should wait for state transition empty result', async () => {
    const promise = waitForStateTransitionResultHandler(call);

    setTimeout(() => {
      tenderDashWsClientMock.emit('tm.event = \'Tx\'', wsMessagesFixture.success);
    }, 10);
    setTimeout(() => {
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, {
        data: { value: { block: { header: { height: '145' } } } },
      });
    }, 10);
    setTimeout(() => {
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, {
        data: { value: { block: { header: { height: '146' } } } },
      });
    }, 10);

    const result = await promise;

    expect(result).to.be.an.instanceOf(WaitForStateTransitionResultResponse);
    expect(result.getV0().getProof()).to.be.undefined();
    expect(result.getV0().getError()).to.be.undefined();
  });

  it('should wait for state transition and return result with proof', async () => {
    requestPayloadMock.getProve.returns(true);

    const promise = waitForStateTransitionResultHandler(call);

    setTimeout(() => {
      tenderDashWsClientMock.emit('tm.event = \'Tx\'', wsMessagesFixture.success);
    }, 10);
    setTimeout(() => {
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, {
        data: { value: { block: { header: { height: '145' } } } },
      });
    }, 10);
    setTimeout(() => {
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, {
        data: { value: { block: { header: { height: '146' } } } },
      });
    }, 10);

    const result = await promise;

    expect(result).to.be.an.instanceOf(WaitForStateTransitionResultResponse);
    expect(result.getV0().getError()).to.be.undefined();
    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.getGrovedbProof());

    const { GetProofsRequestV0 } = GetProofsRequest;
    const getProofsRequest = new GetProofsRequest();
    const { IdentityRequest } = GetProofsRequestV0;

    const identitiesList = stateTransitionFixture.getModifiedDataIds().map((id) => {
      const identityRequest = new IdentityRequest();
      identityRequest.setIdentityId(id.toBuffer());
      identityRequest.setRequestType(IdentityRequest.Type.FULL_IDENTITY);
      return identityRequest;
    });

    getProofsRequest.setV0(
      new GetProofsRequestV0()
        .setIdentitiesList(identitiesList),
    );

    expect(driveClientMock.getProofs).to.be.calledOnceWithExactly(getProofsRequest);
  });

  it('should wait for state transition and return result with error', (done) => {
    waitForStateTransitionResultHandler(call).then((result) => {
      expect(result).to.be.an.instanceOf(WaitForStateTransitionResultResponse);
      expect(result.getV0().getProof()).to.be.undefined();

      const error = result.getV0().getError();
      expect(error).to.be.an.instanceOf(StateTransitionBroadcastError);

      const errorData = error.getData();
      const errorCode = error.getCode();
      const errorMessage = error.getMessage();

      expect(createGrpcErrorFromDriveResponseMock).to.be.calledOnceWithExactly(
        wsMessagesFixture.error.data.value.result.code,
        wsMessagesFixture.error.data.value.result.info,
      );

      expect(errorCode).to.equal(wsMessagesFixture.error.data.value.result.code);
      expect(errorData).to.deep.equal(cbor.encode(errorInfo.metadata));
      expect(errorMessage).to.equal(errorInfo.message);

      done();
    });

    process.nextTick(() => {
      tenderDashWsClientMock.emit('tm.event = \'Tx\'', wsMessagesFixture.error);
    });
  });

  it('should throw an InvalidArgumentGrpcError if stateTransitionHash wasn\'t set', async () => {
    request = new WaitForStateTransitionResultRequest();
    request.setV0(new WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0());

    call.request = WaitForStateTransitionResultRequest.deserializeBinary(request.serializeBinary());

    try {
      await waitForStateTransitionResultHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('state transition hash is not specified');
    }
  });

  it('should throw DeadlineExceededGrpcError after the timeout', async () => {
    const hashString = 'ABFF';

    request = new WaitForStateTransitionResultRequest();
    request.setV0(new WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0());

    const stHash = Buffer.from(hashString, 'hex');

    request.getV0().setStateTransitionHash(stHash);

    transactionNotFoundError.data = `tx (${hashString}) not found, err: %!w(<nil>)`;

    call.request = WaitForStateTransitionResultRequest.deserializeBinary(request.serializeBinary());

    try {
      await waitForStateTransitionResultHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(DeadlineExceededGrpcError);
      expect(e.getMessage()).to.equal(`Waiting period for state transition ${hashString} exceeded`);
      expect(e.getRawMetadata()).to.be.deep.equal({
        stateTransitionHash: hashString,
      });
    }
  });

  it('should throw UnavailableGrpcError if Tenderdash is not available', async () => {
    tenderDashWsClientMock.isConnected = false;

    try {
      await waitForStateTransitionResultHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(UnavailableGrpcError);
      expect(e.getMessage()).to.equal('Tenderdash is not available');
    }
  });
});
