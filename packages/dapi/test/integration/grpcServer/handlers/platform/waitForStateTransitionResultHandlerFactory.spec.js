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
    StateTransitionBroadcastError,
    Proof,
  },
} = require('@dashevo/dapi-grpc');
const getIdentityCreateTransitionFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const { default: loadWasmDpp } = require('@dashevo/wasm-dpp');

const { EventEmitter } = require('events');

const cbor = require('cbor');
const NotFoundGrpcError = require('@dashevo/grpc-common/lib/server/error/NotFoundGrpcError');
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
  let fetchProofForStateTransition;
  let waitForTransactionToBeProvable;
  let transactionNotFoundError;
  let createGrpcErrorFromDriveResponseMock;
  let errorInfo;

  let DashPlatformProtocol;

  before(async () => {
    ({ DashPlatformProtocol } = await loadWasmDpp());
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

    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
      round: 42,
    };

    call = new GrpcCallMock(this.sinon, {
      getStateTransitionHash: this.sinon.stub().returns(hash),
      getProve: this.sinon.stub().returns(false),
    });

    tenderDashWsClientMock = new EventEmitter();
    tenderDashWsClientMock.subscribe = this.sinon.stub();

    const dpp = new DashPlatformProtocol({}, null, null);

    driveClientMock = {
      fetchProofs: this.sinon.stub().resolves({
        identitiesProof: proofFixture,
        metadata: {
          height: 42,
          coreChainLockedHeight: 41,
          timeMs: new Date().getTime(),
          protocolVersion: 1,
        },
      }),
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
    expect(result.getProof()).to.be.undefined();
    expect(result.getError()).to.be.undefined();
  });

  it('should wait for state transition and return result with proof', async () => {
    call.request.getProve.returns(true);

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
    expect(result.getError()).to.be.undefined();
    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getMerkleProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    const { identityIds } = driveClientMock.fetchProofs.firstCall.firstArg;
    expect(identityIds).to.deep.equal(
      stateTransitionFixture.getModifiedDataIds()
        .map((identifier) => identifier.toBuffer()),
    );
  });

  it('should wait for state transition and return result with error', (done) => {
    waitForStateTransitionResultHandler(call).then((result) => {
      expect(result).to.be.an.instanceOf(WaitForStateTransitionResultResponse);
      expect(result.getProof()).to.be.undefined();

      const error = result.getError();
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

    const stHash = Buffer.from(hashString, 'hex');

    request.setStateTransitionHash(stHash);

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
});
