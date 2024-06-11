const EventEmitter = require('events');
const { expect } = require('chai');

const ReconnectableStream = require('../../../lib/transport/ReconnectableStream');
const wait = require('../../../lib/utils/wait');
const logger = require('../../../lib/logger');

describe('ReconnectableStream', () => {
  let reconnectableStream;
  let stream;
  let setTimeoutCallback;
  const setTimeoutReference = 1111;
  const maxRetriesOnError = 2;
  const retryOnErrorDelay = 10;

  beforeEach(function () {
    stream = new EventEmitter();
    stream.cancel = this.sinon.stub()
      .callsFake(() => {
        stream.emit('error', {
          message: 'CANCELED_ON_CLIENT',
          code: 1,
        });
      });

    this.sinon.spy(stream, 'on');
    this.sinon.spy(stream, 'removeListener');
    const createStream = this.sinon.stub()
      .returns(stream);

    reconnectableStream = new ReconnectableStream(createStream, {
      maxRetriesOnError,
      retryOnErrorDelay,
      logger,
    });

    this.sinon.spy(reconnectableStream, 'addListeners');
    this.sinon.spy(reconnectableStream, 'connect');
    this.sinon.spy(reconnectableStream, 'reconnect');
    this.sinon.spy(reconnectableStream, 'endHandler');
    this.sinon.spy(reconnectableStream, 'emit');
    this.sinon.spy(reconnectableStream, 'startAutoReconnect');
    this.sinon.spy(reconnectableStream, 'stopAutoReconnect');
    this.sinon.spy(reconnectableStream, 'retryOnError');
    this.sinon.stub(reconnectableStream, 'clearTimeout');
    this.sinon.stub(reconnectableStream, 'setTimeout')
      .callsFake((callback) => {
        setTimeoutCallback = callback;
        return setTimeoutReference;
      });
  });

  describe('#connect', () => {
    it('should connect and create timeout', async () => {
      const args = [1, 2, 3];
      await reconnectableStream.connect(...args);
      expect(reconnectableStream.args)
        .to
        .deep
        .equal(args);
      expect(reconnectableStream.stream)
        .to
        .equal(stream);
      expect(reconnectableStream.addListeners)
        .to
        .have
        .been
        .calledOnce();
      expect(reconnectableStream.startAutoReconnect)
        .to
        .have
        .been
        .calledOnce();
    });

    it('should not initiate auto reconnect if autoReconnectInterval is not set', async () => {
      reconnectableStream.autoReconnectInterval = 0;
      await reconnectableStream.connect();
      expect(reconnectableStream.startAutoReconnect)
        .to
        .have
        .not
        .been
        .called();
    });
  });

  describe('#startAutoReconnect', () => {
    it('should trigger reconnect after timeout', async () => {
      reconnectableStream.stream = stream;
      reconnectableStream.stream.on('error', () => {
      });

      reconnectableStream.startAutoReconnect();
      setTimeoutCallback();
      expect(reconnectableStream.reconnect)
        .to
        .have
        .been
        .calledOnce();
    });

    it('should not allow starting auto reconnect twice', () => {
      reconnectableStream.startAutoReconnect();
      expect(reconnectableStream.startAutoReconnect.bind(reconnectableStream))
        .to
        .throw('Auto reconnect timeout is already running.');
    });
  });

  describe('#reconnect', () => {
    it('should update connect arguments', async () => {
      reconnectableStream.reconnectTimeout = 1;
      reconnectableStream.stream = stream;
      reconnectableStream.stream.on('error', () => {
      });

      reconnectableStream.on(ReconnectableStream.EVENTS.BEFORE_RECONNECT, (updateArgs) => {
        updateArgs('newArg1', 'newArg2');
      });

      reconnectableStream.reconnect();
      expect(reconnectableStream.connect)
        .to
        .have
        .been
        .calledWith('newArg1', 'newArg2');
    });

    it('should reconnect only if reconnectTimeout exists', () => {
      reconnectableStream.reconnect();
      expect(stream.cancel)
        .to
        .have
        .not
        .been
        .called();
    });

    it('should handle error on reconnect', async () => {
      reconnectableStream.reconnectTimeout = 1;
      reconnectableStream.stream = stream;
      reconnectableStream.stream.on('error', () => {
      });

      const errorEventPromise = new Promise((resolve) => {
        reconnectableStream.on('error', resolve);
      });

      const err = new Error('test error');
      reconnectableStream.createStream.throws(err);
      reconnectableStream.reconnect();
      const emittedError = await errorEventPromise;
      expect(emittedError)
        .to
        .equal(err);
    });
  });

  describe('#addListeners', () => {
    it('should add listeners', async function () {
      reconnectableStream.stream = new EventEmitter();
      this.sinon.spy(reconnectableStream.stream, 'on');
      reconnectableStream.addListeners();
      expect(reconnectableStream.stream.on)
        .to
        .have
        .been
        .calledWith('error');
      expect(reconnectableStream.stream.on)
        .to
        .have
        .been
        .calledWith('data');
      expect(reconnectableStream.stream.on)
        .to
        .have
        .been
        .calledWith('end');
    });

    it('should rewire cancel logic', function () {
      reconnectableStream.stream = new EventEmitter();
      reconnectableStream.stream.cancel = this.sinon.stub();
      this.sinon.spy(reconnectableStream.stream, 'removeListener');
      reconnectableStream.addListeners();
      reconnectableStream.stream.cancel();
      expect(reconnectableStream.stream.removeListener)
        .to
        .have
        .been
        .calledWith('data', reconnectableStream.dataHandler);
      expect(reconnectableStream.stream.removeListener)
        .to
        .have
        .been
        .calledWith('end', reconnectableStream.endHandler);
    });
  });

  describe('#errorHandler', () => {
    it('should handle cancellation', async () => {
      reconnectableStream.errorHandler({
        message: 'CANCELED_ON_CLIENT',
        code: 1,
      });
      expect(reconnectableStream.retryOnError)
        .to
        .have
        .not
        .been
        .called();
    });

    it('should retry on error', function () {
      reconnectableStream.stream = stream;
      reconnectableStream.retryOnError = this.sinon.spy();
      reconnectableStream.errorHandler(new Error('Retry on error'));
      expect(reconnectableStream.retryOnError)
        .to
        .have
        .been
        .calledOnce();
      reconnectableStream.stopAutoReconnect();
    });
  });

  describe('#stopAutoReconnect', async () => {
    it('should stop reconnect timeout', async () => {
      reconnectableStream.reconnectTimeout = setTimeoutReference;
      reconnectableStream.stopAutoReconnect();
      expect(reconnectableStream.clearTimeout)
        .to
        .have
        .been
        .calledOnceWith(setTimeoutReference);
      expect(reconnectableStream.reconnectTimeout)
        .to
        .equal(null);
    });

    it('should do nothing in case reconnect timeout is not set', async () => {
      reconnectableStream.stopAutoReconnect();
      expect(reconnectableStream.clearTimeout)
        .to
        .have
        .not
        .been
        .called();
      expect(reconnectableStream.reconnectTimeout)
        .to
        .equal(null);
    });
  });

  describe('#cancel', async () => {
    it('should cancel stream', async function () {
      reconnectableStream.stream = new EventEmitter();
      reconnectableStream.stream.cancel = this.sinon.stub();
      reconnectableStream.cancel();
      expect(reconnectableStream.stopAutoReconnect)
        .to
        .have
        .been
        .calledOnce();
      expect(reconnectableStream.stream.cancel)
        .to
        .have
        .been
        .calledOnce();
    });
  });

  describe('#endHandler', () => {
    it('should handle end event', async () => {
      reconnectableStream.stream = {};
      reconnectableStream.endHandler();
      expect(reconnectableStream.stopAutoReconnect)
        .to
        .have
        .been
        .called();
      expect(reconnectableStream.emit)
        .to
        .have
        .been
        .calledWith(ReconnectableStream.EVENTS.END);
      expect(reconnectableStream.stream)
        .to
        .equal(null);
    });
  });

  describe('#retryOnError', () => {
    it('should retry in case of an error', async () => {
      reconnectableStream.stream = stream;
      reconnectableStream.retryOnError(new Error('Fake stream error'));

      await wait(10);

      expect(reconnectableStream.stopAutoReconnect)
        .to
        .have
        .been
        .calledOnce();
      expect(reconnectableStream.stream.removeListener)
        .to
        .have
        .been
        .calledWith('end');
      expect(reconnectableStream.stream.removeListener)
        .to
        .have
        .been
        .calledWith('data');
      expect(reconnectableStream.connect)
        .to
        .have
        .been
        .calledOnce();
    });

    it('should manage args via beforeReconnect', async () => {
      reconnectableStream.stream = stream;

      const newArgs = ['newArg1', 'newArg2'];
      reconnectableStream.on('beforeReconnect', (updateArgs) => {
        updateArgs(newArgs);
      });

      reconnectableStream.retryOnError(new Error('Fake stream error'));

      await wait(10);

      expect(reconnectableStream.stopAutoReconnect)
        .to
        .have
        .been
        .calledOnce();
      expect(reconnectableStream.stream.removeListener)
        .to
        .have
        .been
        .calledWith('end');
      expect(reconnectableStream.stream.removeListener)
        .to
        .have
        .been
        .calledWith('data');
      expect(reconnectableStream.connect)
        .to
        .have
        .been
        .calledOnceWith(newArgs);
    });

    it('should handle error in case retry attempts were exhausted', async () => {
      reconnectableStream.stream = stream;

      // Exhaust retry attempts
      for (let i = 0; i < maxRetriesOnError; i += 1) {
        const error = new Error(`Retry exhaust error ${i}`);
        reconnectableStream.retryOnError(error);
        // eslint-disable-next-line no-await-in-loop
        await wait(10);
      }

      let lastError;
      reconnectableStream.on(ReconnectableStream.EVENTS.ERROR, (error) => {
        lastError = error;
      });

      const error = new Error('Last error');
      reconnectableStream.retryOnError(error);

      expect(lastError)
        .to
        .equal(error);
    });

    it('should handle retry error', async () => {
      reconnectableStream.stream = stream;

      const retryError = new Error('Error retrying on error');

      // Prepare createStream to throw an error on retry attempt
      reconnectableStream.createStream.throws(retryError);

      // Emit stream error
      reconnectableStream.retryOnError(
        new Error('Fake stream error'),
      );

      // Wait for
      await wait(10);

      expect(reconnectableStream.stopAutoReconnect)
        .to
        .have
        .been
        .calledOnce();
      expect(reconnectableStream.emit)
        .to
        .have
        .been
        .calledWith(
          'error',
          retryError,
        );
    });
  });
});
