const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');

chai.use(chaiAsPromised);
chai.should();

const ZMQClient = require('@dashevo/dashd-zmq');
const EventEmitter = require('events');
const ensureBlock = require('../../../lib/core/ensureBlock');

describe('ensureBlock', () => {
  const hash = '00000';
  const otherHash = '00001';
  const socketClient = new EventEmitter();
  let rpcClient;

  beforeEach(function beforeEach() {
    socketClient.subscribe = this.sinon.stub();

    rpcClient = {
      getBlock: this.sinon.stub().resolves(true),
    };
  });

  it('should ensure a block exist before returning promise', async () => {
    await ensureBlock(socketClient, rpcClient, hash);

    expect(rpcClient.getBlock).to.be.calledOnceWithExactly(hash);
  });

  it('should wait for block if not found before returning promise', (done) => {
    const err = new Error();
    err.code = -5;
    err.message = 'Block not found';

    rpcClient.getBlock.throws(err);

    ensureBlock(socketClient, rpcClient, hash).then(done);

    setImmediate(() => {
      socketClient.emit(ZMQClient.TOPICS.hashblock, otherHash);
    });

    setImmediate(() => {
      socketClient.emit(ZMQClient.TOPICS.hashblock, hash);
    });

    expect(rpcClient.getBlock).to.be.calledOnceWithExactly(hash);
  });

  it('should throw on unexpected error', async () => {
    const err = new Error();
    err.code = -6;
    err.message = 'Another error';

    rpcClient.getBlock.throws(err);

    try {
      await ensureBlock(socketClient, rpcClient, hash);
      expect.fail('Internal error must be thrown');
    } catch (e) {
      expect(e).to.equal(err);
      expect(rpcClient.getBlock).to.be.calledOnceWithExactly(hash);
    }
  });
});
