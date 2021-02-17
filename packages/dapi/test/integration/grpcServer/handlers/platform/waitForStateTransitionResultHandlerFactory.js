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
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const { EventEmitter } = require('events');

const cbor = require('cbor');
const BlockchainListener = require('../../../../../lib/externalApis/tenderdash/blockchainListener/BlockchainListener');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');
const fetchProofForStateTransitionFactory = require('../../../../../lib/externalApis/drive/fetchProofForStateTransitionFactory');
const waitForTransactionToBeProvableFactory = require('../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/waitForTransactionToBeProvableFactory');
const waitForTransactionResult = require('../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/waitForTransactionResult');
const waitForTransactionCommitment = require('../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/waitForTransactionCommitment');

const waitForStateTransitionResultHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/waitForStateTransitionResultHandlerFactory');

describe('waitForStateTransitionResultHandlerFactory', () => {
  let call;
  let waitForStateTransitionResultHandler;
  let driveClinetMock;
  let tenderDashWsClientMock;
  let blockchainListener;
  let dppMock;
  let hash;
  let proofFixture;
  let wsMessagesFixture;
  let stateTransitionFixture;
  let request;
  let emptyBlockFixture;
  let blockWithTxFixture;
  let fetchProofForStateTransition;
  let waitForTransactionToBeProvable;

  beforeEach(function beforeEach() {
    hash = Buffer.from('56458F2D8A8617EA322931B72C103CDD93820004E534295183A6EF215B93C76E', 'hex');

    wsMessagesFixture = {
      success: {
        query: "tm.event = 'Tx'",
        data: {
          type: 'tendermint/event/Tx',
          value: {
            TxResult: {
              height: '145',
              tx: 'pWR0eXBlA2lhc3NldExvY2ujZXByb29momR0eXBlAGtpbnN0YW50TG9ja1ilAR272lhhsS11I/IKpeDUL1LePc0tXC/pGbpntZ8FDSBuAAAAAHvUKCicVybMXMiWz60mTKDN2H7HesE1zhNhy9w+zKjYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAa291dHB1dEluZGV4AGt0cmFuc2FjdGlvbljfAwAAAAFft1DH/7MLyiiZTQ0v9kxxx5IO+g3OowKiGXGr/gzTXAEAAABrSDBFAiEA9zBXt5ZbkZ0miGrXtJPF9abrNUHafXIGRHXeritMEZECIBO0nrmvNv/jff27bDehIf3kD+WHQACWj5UvryJNQvyAASECG117xwKATG95Jur1SvBo/vAjYHx5AnYYOwsN3zL8Wyf/////AgEAAAAAAAAAFmoU7MiTGZFsxDcto0FsSOKqkcWmk/5OiAAAAAAAABl2qRTk6MFuEOFzT3vBIbU1Hio2UuiDzYisAAAAAGlzaWduYXR1cmVYQSANwCdg67KHh/OiSv9FW8qNFj+8OBvwnm3Ybg2Ju0tGNmkw3jAkdOgHLqAkmHCtiSvqZ7IhGDXhU5YtHCk6PIOIamlkZW50aXR5SWRYIJmUCrEaSl7bW6UkE3rBhlQjTBhJ4v1m0ORUXh434DTDb3Byb3RvY29sVmVyc2lvbgA=',
              result: {},
            },
          },
        },
        events: {
          'tx.hash': [
            '56458F2D8A8617EA322931B72C103CDD93820004E534295183A6EF215B93C76E',
          ],
          'tx.height': [
            '145',
          ],
          'tm.event': [
            'Tx',
          ],
        },
      },
      error: {
        query: "tm.event = 'Tx'",
        data: {
          type: 'tendermint/event/Tx',
          value: {
            TxResult: {
              height: '135',
              tx: 'pWR0eXBlAmlhc3NldExvY2ujZXByb29momR0eXBlAGtpbnN0YW50TG9ja1ilAR272lhhsS11I/IKpeDUL1LePc0tXC/pGbpntZ8FDSBuAAAAAMfKlZZZ3oAHaxO0bEIYXCSEpwTuR/baTwASqjgFgDAGAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAa291dHB1dEluZGV4AGt0cmFuc2FjdGlvbljfAwAAAAFl5SQeBBDkK7Us9JcOU+Gp1oi4NIl/01A+5GAKeHi2JwEAAABrSDBFAiEAq9XMPgtU9J0imH6YJ/RtbxwJsavuhIpECU5Lw9h0xpoCIEgkU1njDQCe06YqRyeVYc6wK8G7Y/M5X+XicfJKo5P6ASEDK3jwtdIToEQAgTPMXxpjon4geQaNbbRNT/Xz50UgdHH/////AgEAAAAAAAAAFmoU8HHK+aRqNJOWXjNlOO3iWwvV45CDkAAAAAAAABl2qRTFVGzrfaB6ZhmvE8h2unBNgcJIMIisAAAAAGlzaWduYXR1cmVYQR/OHDEQUcSxczLBvMP9Z0HmRaDoCS6tTyFLbWhn7bAfJTlPF9hIbh13260WSCiDceJjWaYB0JuOGsqu2ZB5F0dDanB1YmxpY0tleXOBo2JpZABkZGF0YVghA85GJWE321+kW0HIwl3M6wO9BIHDxY80HlQgc1wRalT5ZHR5cGUAb3Byb3RvY29sVmVyc2lvbgA=',
              result: {
                code: 2,
                log: '{"error":{"message":"Invalid state transition","data":{"errors":[{"name":"IdentityPublicKeyAlreadyExistsError","message":"Identity public key already exists","publicKeyHash":{"type":"Buffer","data":[216,100,221,206,173,76,253,13,66,247,118,172,153,161,161,189,154,91,240,205]}}]}}}',
              },
            },
          },
        },
        events: {
          'tm.event': [
            'Tx',
          ],
          'tx.hash': [
            '56458F2D8A8617EA322931B72C103CDD93820004E534295183A6EF215B93C76E',
          ],
          'tx.height': [
            '135',
          ],
        },
      },
    };
    emptyBlockFixture = {
      data: { value: { block: { data: { txs: [] } } } },
    };
    blockWithTxFixture = {
      data: {
        value: {
          block: {
            data: {
              txs: [
                wsMessagesFixture.success.data.value.TxResult.tx,
              ],
            },
          },
        },
      },
    };

    proofFixture = {
      rootTreeProof: Buffer.alloc(1, 1),
      storeTreeProof: Buffer.alloc(1, 2),
    };

    call = new GrpcCallMock(this.sinon, {
      getStateTransitionHash: this.sinon.stub().returns(hash),
      getProve: this.sinon.stub().returns(false),
    });

    tenderDashWsClientMock = new EventEmitter();
    tenderDashWsClientMock.subscribe = this.sinon.stub();

    stateTransitionFixture = getIdentityCreateTransitionFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock.stateTransition.createFromBuffer.resolves(stateTransitionFixture);

    driveClinetMock = {
      fetchProofs: this.sinon.stub().resolves({ identitiesProof: proofFixture }),
    };

    blockchainListener = new BlockchainListener(tenderDashWsClientMock);
    blockchainListener.start();

    fetchProofForStateTransition = fetchProofForStateTransitionFactory(driveClinetMock);

    waitForTransactionToBeProvable = waitForTransactionToBeProvableFactory(
      waitForTransactionResult,
      waitForTransactionCommitment,
    );

    waitForStateTransitionResultHandler = waitForStateTransitionResultHandlerFactory(
      fetchProofForStateTransition,
      waitForTransactionToBeProvable,
      blockchainListener,
      dppMock,
      1000,
    );
  });

  it('should wait for state transition empty result', async () => {
    const promise = waitForStateTransitionResultHandler(call);

    setTimeout(() => {
      tenderDashWsClientMock.emit('tm.event = \'Tx\'', wsMessagesFixture.success);
    }, 10);
    setTimeout(() => {
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, blockWithTxFixture);
    }, 10);
    setTimeout(() => {
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, emptyBlockFixture);
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
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, blockWithTxFixture);
    }, 10);
    setTimeout(() => {
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, emptyBlockFixture);
    }, 10);

    const result = await promise;

    expect(result).to.be.an.instanceOf(WaitForStateTransitionResultResponse);
    expect(result.getError()).to.be.undefined();
    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const rootTreeProof = proof.getRootTreeProof();
    const storeTreeProof = proof.getStoreTreeProof();

    expect(rootTreeProof).to.deep.equal(proofFixture.rootTreeProof);
    expect(storeTreeProof).to.deep.equal(proofFixture.storeTreeProof);

    expect(driveClinetMock.fetchProofs).to.be.calledOnceWithExactly({
      identityIds: stateTransitionFixture.getModifiedDataIds(),
    });
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

      const { error: abciError } = JSON.parse(
        wsMessagesFixture.error.data.value.TxResult.result.log,
      );

      expect(errorCode).to.equal(wsMessagesFixture.error.data.value.TxResult.result.code);
      expect(errorData).to.deep.equal(cbor.encode(abciError.data));
      expect(errorMessage).to.equal(abciError.message);

      done();
    });

    process.nextTick(() => {
      tenderDashWsClientMock.emit('tm.event = \'Tx\'', wsMessagesFixture.error);
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, blockWithTxFixture);
      tenderDashWsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, emptyBlockFixture);
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
    request = new WaitForStateTransitionResultRequest();
    const stHash = Buffer.from('abff', 'hex');
    request.setStateTransitionHash(stHash);

    call.request = WaitForStateTransitionResultRequest.deserializeBinary(request.serializeBinary());

    try {
      await waitForStateTransitionResultHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(DeadlineExceededGrpcError);
      expect(e.getMessage()).to.equal('Waiting period for state transition ABFF exceeded');
      expect(e.getRawMetadata()).to.be.deep.equal({
        stateTransitionHash: 'ABFF',
      });
    }
  });
});
