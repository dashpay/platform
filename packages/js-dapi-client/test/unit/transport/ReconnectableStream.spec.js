const EventEmitter = require('events');
const { expect } = require('chai');

const ReconnectableStream = require('../../../lib/transport/ReconnectableStream');

describe('ReconnectableStream', () => {
  let reconnectableStream;
  let stream;
  let timeoutCallback;

  beforeEach(function () {
    stream = new EventEmitter();
    stream.cancel = this.sinon.stub().callsFake(() => {
      stream.emit('error', {
        message: 'CANCELED_ON_CLIENT',
        code: 1,
      });
    });
    stream.destroy = this.sinon.stub().callsFake((e) => {
      stream.emit('error', e);
    });
    this.sinon.spy(stream, 'on');
    const streamFunction = this.sinon.stub().returns(stream);

    reconnectableStream = new ReconnectableStream(streamFunction);

    this.sinon.spy(reconnectableStream, 'addListeners');
    this.sinon.spy(reconnectableStream, 'connect');
    this.sinon.spy(reconnectableStream, 'reconnect');
    this.sinon.spy(reconnectableStream, 'endHandler');
    this.sinon.spy(reconnectableStream, 'emit');
    this.sinon.spy(reconnectableStream, 'stopReconnectTimeout');
    this.sinon.stub(global, 'setTimeout').callsFake((callback) => {
      timeoutCallback = callback;
      return 1;
    });
    this.sinon.stub(global, 'clearTimeout');
  });

  describe('#connect', () => {
    it('should connect and create timeout', async () => {
      await reconnectableStream.connect();
      expect(reconnectableStream.addListeners).to.have.been.calledOnce();
      expect(reconnectableStream.reconnectTimeout).to.exist();
    });

    it('should trigger reconnect after timeout', async () => {
      await reconnectableStream.connect();
      timeoutCallback();
      expect(reconnectableStream.reconnect).to.have.been.calledOnce();
    });
  });

  describe('#reconnect', () => {
    it('should update connect arguments', async () => {
      await reconnectableStream.connect();

      reconnectableStream.on(ReconnectableStream.EVENTS.BEFORE_RECONNECT, (updateArgs) => {
        updateArgs('newArg1', 'newArg2');
      });

      reconnectableStream.reconnect();
      expect(reconnectableStream.connect).to.have.been.calledWith('newArg1', 'newArg2');
    });

    it('should handle error on reconnect', async () => {
      await reconnectableStream.connect();
      const errorEventPromise = new Promise((resolve) => {
        reconnectableStream.on('error', resolve);
      });

      const err = new Error('test error');
      reconnectableStream.streamFunction.throws(err);
      reconnectableStream.reconnect();
      const emittedError = await errorEventPromise;
      expect(emittedError).to.equal(err);
    });
  });

  describe('#addListeners', () => {
    it('should add listeners', async function () {
      reconnectableStream.stream = new EventEmitter();
      this.sinon.spy(reconnectableStream.stream, 'on');
      reconnectableStream.addListeners();
      expect(reconnectableStream.stream.on).to.have.been.calledWith('error');
      expect(reconnectableStream.stream.on).to.have.been.calledWith('data');
      expect(reconnectableStream.stream.on).to.have.been.calledWith('end');
    });

    it('should rewire cancel logic', async function () {
      reconnectableStream.stream = new EventEmitter();
      reconnectableStream.stream.cancel = this.sinon.stub();
      this.sinon.spy(reconnectableStream.stream, 'removeListener');
      reconnectableStream.addListeners();
      reconnectableStream.stream.cancel();
      expect(reconnectableStream.stream.removeListener)
        .to.have.been.calledWith('data', reconnectableStream.dataHandler);
      expect(reconnectableStream.stream.removeListener)
        .to.have.been.calledWith('end', reconnectableStream.endHandler);
    });
  });

  describe('#errorHandler', () => {
    it('should handle error', async () => {
      await reconnectableStream.connect();
      let emittedError;
      reconnectableStream.on(ReconnectableStream.EVENTS.ERROR, (error) => {
        emittedError = error;
      });

      const error = new Error('test error');
      stream.emit('error', error);

      expect(emittedError).to.equal(error);
    });

    it('should handle cancellation', async () => {
      await reconnectableStream.connect();
      stream.cancel();
      expect(reconnectableStream.emit).to.have.not.been.called();
    });
  });

  describe('#stopReconnectTimeout', async () => {
    it('should stop reconnect timeout', async () => {
      await reconnectableStream.connect();
      reconnectableStream.stopReconnectTimeout();
      expect(global.clearTimeout).to.have.been.calledOnce();
      expect(reconnectableStream.reconnectTimeout).to.equal(null);
    });

    it('should do nothing in case reconnect timeout is not set', async () => {
      await reconnectableStream.connect();
      timeoutCallback();
      reconnectableStream.stopReconnectTimeout();
      expect(global.clearTimeout).to.have.not.been.called();
      expect(reconnectableStream.reconnectTimeout).to.equal(null);
    });
  });

  describe('#cancel', async () => {
    it('should cancel stream', async function () {
      reconnectableStream.stream = new EventEmitter();
      reconnectableStream.stream.cancel = this.sinon.stub();
      reconnectableStream.cancel();
      expect(reconnectableStream.stopReconnectTimeout).to.have.been.calledOnce();
      expect(reconnectableStream.stream.cancel).to.have.been.calledOnce();
    });
  });

  describe('#destroy', async () => {
    it('should destroy stream', async () => {
      await reconnectableStream.connect();
      const errorEventPromise = new Promise((resolve) => {
        reconnectableStream.on('error', resolve);
      });

      const err = new Error('test error');
      reconnectableStream.destroy(err);

      const emittedError = await errorEventPromise;
      expect(emittedError).to.equal(err);
      expect(reconnectableStream.stopReconnectTimeout).to.have.been.calledOnce();
      expect(stream.destroy).to.have.been.calledOnceWith(err);
    });
  });

  describe('#endHandler', () => {
    it('should handle end event', async () => {
      reconnectableStream.endHandler();
      expect(reconnectableStream.stopReconnectTimeout).to.have.been.called();
      expect(reconnectableStream.emit).to.have.been.calledWith(ReconnectableStream.EVENTS.END);
      expect(reconnectableStream.stream).to.equal(null);
    });
  });
});
