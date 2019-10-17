const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const cbor = require('cbor');

const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  UpdateStateTransitionResponse,
} = require('@dashevo/dapi-grpc');

const getStPacketFixture = require('../../../../../lib/test/fixtures/getStPacketFixture');
const getStHeaderFixture = require('../../../../../lib/test/fixtures/getStHeaderFixture');
const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const updateStateHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/core/updateStateHandlerFactory',
);

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

describe('updateStateHandlerFactory', () => {
  let call;
  let rpcClientMock;
  let updateStateHandler;
  let response;
  let stHeader;
  let stPacket;
  let log;

  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }

    const stPacketFixture = getStPacketFixture();
    const stHeaderFixture = getStHeaderFixture(stPacketFixture);

    stHeader = Buffer.from(stHeaderFixture.serialize(), 'hex');
    stPacket = stPacketFixture.serialize();

    call = new GrpcCallMock(this.sinon, {
      getHeader: this.sinon.stub().returns(stHeader),
      getPacket: this.sinon.stub().returns(stPacket),
    });

    log = JSON.stringify({
      error: {
        message: 'some message',
        data: {
          error: 'some data',
        },
      },
    });

    const code = 0;

    response = {
      id: '',
      jsonrpc: '2.0',
      error: '',
      result: {
        check_tx: { code, log },
        deliver_tx: { code, log },
        hash:
        'B762539A7C17C33A65C46727BFCF2C701390E6AD7DE5190B6CC1CF843CA7E262',
        height: '24',
      },
    };

    rpcClientMock = {
      request: this.sinon.stub().resolves(response),
    };

    updateStateHandler = updateStateHandlerFactory(
      rpcClientMock,
    );
  });

  afterEach(function afterEach() {
    this.sinon.restore();
  });

  it('should throw an InvalidArgumentGrpcError if header is not specified', async () => {
    call.request.getHeader.returns(null);

    try {
      await updateStateHandler(call);

      expect.fail('InvalidArgumentGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid argument: header is not specified');
      expect(rpcClientMock.request).to.not.be.called();
    }
  });

  it('should throw an InvalidArgumentGrpcError if packet is not specified', async () => {
    call.request.getPacket.returns(null);

    try {
      await updateStateHandler(call);

      expect.fail('InvalidArgumentGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid argument: packet is not specified');
      expect(rpcClientMock.request).to.not.be.called();
    }
  });

  it('should return valid result', async () => {
    const result = await updateStateHandler(call);

    const st = {
      header: Buffer.from(stHeader).toString('hex'),
      packet: Buffer.from(stPacket),
    };

    const tx = cbor.encodeCanonical(st).toString('base64');

    expect(result).to.be.an.instanceOf(UpdateStateTransitionResponse);
    expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
  });

  it('should throw InvalidArgumentGrpcError if Tendermint Core returns check_tx with non zero code', async () => {
    response.result.check_tx.code = 1;

    const st = {
      header: Buffer.from(stHeader).toString('hex'),
      packet: Buffer.from(stPacket),
    };

    const tx = cbor.encodeCanonical(st).toString('base64');

    try {
      await updateStateHandler(call);

      expect.fail('InvalidArgumentGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid argument: some message');
      expect(e.getMetadata()).to.deep.equal({ error: 'some data' });
      expect(rpcClientMock.request).to.be.calledOnceWith('broadcast_tx_commit', { tx });
    }
  });

  it('should throw InvalidArgumentGrpcError if Tendermint Core returns deliver_tx with non zero code', async () => {
    response.result.deliver_tx.code = 1;

    try {
      await updateStateHandler(call);

      expect.fail('InvalidArgumentGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid argument: some message');
      expect(e.getMetadata()).to.deep.equal({ error: 'some data' });
    }
  });

  it('should return error if timeout happened');

  it('should return InternalGrpcError if Tendermint Core throws an error', async () => {
    const error = {
      code: -32603,
      message: 'Internal error',
      data: 'just error',
    };

    rpcClientMock.request.resolves({
      id: '',
      jsonrpc: '2.0',
      error,
    });

    try {
      await updateStateHandler(call);

      expect.fail('InternalGrpcError was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InternalGrpcError);
      expect(e.getError()).to.deep.equal(error);
    }
  });
});
