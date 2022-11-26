const EventEmitter = require('events');
const { expect } = require('chai');

const ReconnectableStream = require('../../../lib/transport/ReconnectableStream');
const wait = require('../../../lib/utils/wait');

describe('ReconnectableStream', () => {
  let reconnectableStream;
  let stream;
  let setTimeoutCallback;
  const maxRetriesOnError = 2;
  const retryOnErrorDelay = 10;

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

    reconnectableStream = new ReconnectableStream(streamFunction, {
      maxRetriesOnError,
      retryOnErrorDelay,
    });

    this.sinon.spy(reconnectableStream, 'addListeners');
    this.sinon.spy(reconnectableStream, 'connect');
    this.sinon.spy(reconnectableStream, 'reconnect');
    this.sinon.spy(reconnectableStream, 'endHandler');
    this.sinon.spy(reconnectableStream, 'emit');
    this.sinon.spy(reconnectableStream, 'stopReconnectTimeout');
    this.sinon.stub(reconnectableStream, 'clearTimeout');
    this.sinon.stub(reconnectableStream, 'setTimeout').callsFake((callback) => {
      setTimeoutCallback = callback;
      return 1;
    });
  });

  describe('#connect', () => {
    it('should connect and create timeout', async () => {
      await reconnectableStream.connect();
      expect(reconnectableStream.addListeners).to.have.been.calledOnce();
      expect(reconnectableStream.reconnectTimeout).to.exist();
    });

    it('should trigger reconnect after timeout', async () => {
      await reconnectableStream.connect();
      setTimeoutCallback();
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
    it('should handle cancellation', async () => {
      await reconnectableStream.connect();
      stream.cancel();
      expect(reconnectableStream.emit).to.have.not.been.called();
    });

    it('should retry in case of an error', async () => {
      await reconnectableStream.connect();
      const newArgs = ['newArg1', 'newArg2'];
      reconnectableStream.on('beforeReconnect', (updateArgs) => {
        updateArgs(newArgs);
      });

      reconnectableStream.stream.emit('error', new Error('Fake stream error'));
      setTimeoutCallback(); // Simulate setTimeout execution

      await wait(10);

      expect(reconnectableStream.connect).to.have.been.calledTwice();
      expect(reconnectableStream.connect.secondCall.args)
        .to.deep.equal([newArgs]);
    });

    it('should handle error in case retry attempts were exhausted', async () => {
      await reconnectableStream.connect();

      // Exhaust retry attempts
      for (let i = 0; i < maxRetriesOnError; i += 1) {
        const error = new Error(`Retry exhaust error ${i}`);
        reconnectableStream.stream.emit('error', error);
        // eslint-disable-next-line no-await-in-loop
        await wait(10);
      }

      let lastError;
      reconnectableStream.on(ReconnectableStream.EVENTS.ERROR, (error) => {
        lastError = error;
      });

      const error = new Error('Last error');
      stream.emit('error', error);

      expect(lastError).to.equal(error);
    });

    it('should handle retry error', async () => {
      await reconnectableStream.connect();

      const retryError = new Error('Error retrying on error');

      // Prepare streamFunction to throw an error on retry attempt
      reconnectableStream.streamFunction.throws(retryError);

      // Emit stream error
      reconnectableStream.stream.emit(
        'error',
        new Error('Fake stream error'),
      );

      // Wait for
      await wait(10);

      expect(reconnectableStream.emit).to.have.been.calledWith(
        'error',
        retryError,
      );
    });
  });

  describe('#stopReconnectTimeout', async () => {
    it('should stop reconnect timeout', async () => {
      await reconnectableStream.connect();
      reconnectableStream.stopReconnectTimeout();
      expect(reconnectableStream.clearTimeout).to.have.been.calledOnce();
      expect(reconnectableStream.reconnectTimeout).to.equal(null);
    });

    it('should do nothing in case reconnect timeout is not set', async () => {
      await reconnectableStream.connect();
      setTimeoutCallback();
      reconnectableStream.stopReconnectTimeout();
      expect(reconnectableStream.clearTimeout).to.have.not.been.called();
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
      // Do not retry on destroy
      reconnectableStream.maxRetriesOnError = 0;
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
