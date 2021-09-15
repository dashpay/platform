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
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const { EventEmitter } = require('events');

const cbor = require('cbor');
const NotFoundGrpcError = require('@dashevo/grpc-common/lib/server/error/NotFoundGrpcError');
const BlockchainListener = require('../../../../../lib/externalApis/tenderdash/BlockchainListener');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');
const fetchProofForStateTransitionFactory = require('../../../../../lib/externalApis/drive/fetchProofForStateTransitionFactory');
const waitForTransactionToBeProvableFactory = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionToBeProvableFactory');
const waitForTransactionResult = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionResult');

const waitForStateTransitionResultHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/waitForStateTransitionResultHandlerFactory');
const waitForHeightFactory = require('../../../../../lib/externalApis/tenderdash/waitForHeightFactory');

describe('waitForStateTransitionResultHandlerFactory', () => {
  let call;
  let waitForStateTransitionResultHandler;
  let driveClientMock;
  let tenderDashWsClientMock;
  let blockchainListener;
  let dppMock;
  let hash;
  let proofFixture;
  let wsMessagesFixture;
  let stateTransitionFixture;
  let request;
  let fetchProofForStateTransition;
  let waitForTransactionToBeProvable;
  let transactionNotFoundError;
  let storeTreeProofs;
  let createGrpcErrorFromDriveResponseMock;
  let errorInfo;

  beforeEach(function beforeEach() {
    const hashString = '56458F2D8A8617EA322931B72C103CDD93820004E534295183A6EF215B93C76E';
    hash = Buffer.from(hashString, 'hex');

    errorInfo = {
      message: 'Identity not found',
      metadata: {
        error: 'some data',
      },
    };

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
                code: 1043,
                info: cbor.encode(errorInfo).toString('base64'),
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

    proofFixture = {
      rootTreeProof: Buffer.alloc(1, 1),
      storeTreeProof: Buffer.alloc(1, 2),
    };

    storeTreeProofs = new StoreTreeProofs();
    storeTreeProofs.setIdentitiesProof(proofFixture.storeTreeProof);

    call = new GrpcCallMock(this.sinon, {
      getStateTransitionHash: this.sinon.stub().returns(hash),
      getProve: this.sinon.stub().returns(false),
    });

    tenderDashWsClientMock = new EventEmitter();
    tenderDashWsClientMock.subscribe = this.sinon.stub();

    stateTransitionFixture = getIdentityCreateTransitionFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock.stateTransition.createFromBuffer.resolves(stateTransitionFixture);

    driveClientMock = {
      fetchProofs: this.sinon.stub().resolves({
        identitiesProof: proofFixture,
        metadata: {
          height: 42,
          coreChainLockedHeight: 41,
        },
      }),
    };

    blockchainListener = new BlockchainListener(tenderDashWsClientMock);
    blockchainListener.start();

    fetchProofForStateTransition = fetchProofForStateTransitionFactory(driveClientMock);

    const waitForHeight = waitForHeightFactory(
      blockchainListener,
    );

    transactionNotFoundError = new Error();

    transactionNotFoundError.code = -32603;
    transactionNotFoundError.data = `tx (${hashString}) not found`;

    const getExistingTransactionResult = this.sinon.stub().rejects(transactionNotFoundError);

    waitForTransactionToBeProvable = waitForTransactionToBeProvableFactory(
      waitForTransactionResult,
      getExistingTransactionResult,
      waitForHeight,
    );

    createGrpcErrorFromDriveResponseMock = this.sinon.stub().returns(
      new NotFoundGrpcError(errorInfo.message, errorInfo.metadata),
    );

    waitForStateTransitionResultHandler = waitForStateTransitionResultHandlerFactory(
      fetchProofForStateTransition,
      waitForTransactionToBeProvable,
      blockchainListener,
      dppMock,
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
    const rootTreeProof = proof.getRootTreeProof();
    const resultStoreTreeProof = proof.getStoreTreeProofs();

    expect(rootTreeProof).to.deep.equal(proofFixture.rootTreeProof);
    expect(resultStoreTreeProof).to.deep.equal(storeTreeProofs);

    expect(driveClientMock.fetchProofs).to.be.calledOnceWithExactly({
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

      expect(createGrpcErrorFromDriveResponseMock).to.be.calledOnceWithExactly(
        wsMessagesFixture.error.data.value.TxResult.result.code,
        wsMessagesFixture.error.data.value.TxResult.result.info,
      );

      expect(errorCode).to.equal(wsMessagesFixture.error.data.value.TxResult.result.code);
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

    transactionNotFoundError.data = `tx (${hashString}) not found`;

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
