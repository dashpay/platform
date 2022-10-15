const EventEmitter = require('events');
const { expect } = require('chai');

const DAPIStream = require('../../../lib/transport/DAPIStream');

describe('DAPIStream', () => {
  let dapiStream;
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

    dapiStream = new DAPIStream(streamFunction);

    this.sinon.spy(dapiStream, 'addListeners');
    this.sinon.spy(dapiStream, 'connect');
    this.sinon.spy(dapiStream, 'reconnect');
    this.sinon.spy(dapiStream, 'endHandler');
    this.sinon.spy(dapiStream, 'emit');
    this.sinon.spy(dapiStream, 'stopReconnectTimeout');
    this.sinon.stub(global, 'setTimeout').callsFake((callback) => {
      timeoutCallback = callback;
      return 1;
    });
    this.sinon.stub(global, 'clearTimeout');
  });

  describe('#connect', () => {
    it('should connect and create timeout', async () => {
      await dapiStream.connect();
      expect(dapiStream.addListeners).to.have.been.calledOnce();
      expect(dapiStream.reconnectTimeout).to.exist();
    });

    it('should trigger reconnect after timeout', async () => {
      await dapiStream.connect();
      timeoutCallback();
      expect(dapiStream.reconnect).to.have.been.calledOnce();
    });
  });

  describe('#reconnect', () => {
    it('should update connect arguments', async () => {
      await dapiStream.connect();

      dapiStream.on(DAPIStream.EVENTS.BEFORE_RECONNECT, (updateArgs) => {
        updateArgs('newArg1', 'newArg2');
      });

      dapiStream.reconnect();
      expect(dapiStream.connect).to.have.been.calledWith('newArg1', 'newArg2');
    });

    it('should handle error on reconnect', async () => {
      await dapiStream.connect();
      const errorEventPromise = new Promise((resolve) => {
        dapiStream.on('error', resolve);
      });

      const err = new Error('test error');
      dapiStream.streamFunction.throws(err);
      dapiStream.reconnect();
      const emittedError = await errorEventPromise;
      expect(emittedError).to.equal(err);
    });
  });

  describe('#addListeners', () => {
    it('should add listeners', async function () {
      dapiStream.stream = new EventEmitter();
      this.sinon.spy(dapiStream.stream, 'on');
      dapiStream.addListeners();
      expect(dapiStream.stream.on).to.have.been.calledWith('error');
      expect(dapiStream.stream.on).to.have.been.calledWith('data');
      expect(dapiStream.stream.on).to.have.been.calledWith('end');
    });

    it('should rewire cancel logic', async function () {
      dapiStream.stream = new EventEmitter();
      dapiStream.stream.cancel = this.sinon.stub();
      this.sinon.spy(dapiStream.stream, 'removeListener');
      dapiStream.addListeners();
      dapiStream.stream.cancel();
      expect(dapiStream.stream.removeListener)
        .to.have.been.calledWith('data', dapiStream.dataHandler);
      expect(dapiStream.stream.removeListener)
        .to.have.been.calledWith('end', dapiStream.endHandler);
    });
  });

  describe('#errorHandler', () => {
    it('should handle error', async () => {
      await dapiStream.connect();
      let emittedError;
      dapiStream.on(DAPIStream.EVENTS.ERROR, (error) => {
        emittedError = error;
      });

      const error = new Error('test error');
      stream.emit('error', error);

      expect(emittedError).to.equal(error);
    });

    it('should handle cancellation', async () => {
      await dapiStream.connect();
      stream.cancel();
      expect(dapiStream.emit).to.have.not.been.called();
    });
  });

  describe('#stopReconnectTimeout', async () => {
    it('should stop reconnect timeout', async () => {
      await dapiStream.connect();
      dapiStream.stopReconnectTimeout();
      expect(global.clearTimeout).to.have.been.calledOnce();
      expect(dapiStream.reconnectTimeout).to.equal(null);
    });

    it('should do nothing in case reconnect timeout is not set', async () => {
      await dapiStream.connect();
      timeoutCallback();
      dapiStream.stopReconnectTimeout();
      expect(global.clearTimeout).to.have.not.been.called();
      expect(dapiStream.reconnectTimeout).to.equal(null);
    });
  });

  describe('#cancel', async () => {
    it('should cancel stream', async function () {
      dapiStream.stream = new EventEmitter();
      dapiStream.stream.cancel = this.sinon.stub();
      dapiStream.cancel();
      expect(dapiStream.stopReconnectTimeout).to.have.been.calledOnce();
      expect(dapiStream.stream.cancel).to.have.been.calledOnce();
    });
  });

  describe('#destroy', async () => {
    it('should destroy stream', async () => {
      await dapiStream.connect();
      const errorEventPromise = new Promise((resolve) => {
        dapiStream.on('error', resolve);
      });

      const err = new Error('test error');
      dapiStream.destroy(err);

      const emittedError = await errorEventPromise;
      expect(emittedError).to.equal(err);
      expect(dapiStream.stopReconnectTimeout).to.have.been.calledOnce();
      expect(stream.destroy).to.have.been.calledOnceWith(err);
    });
  });

  describe('#endHandler', () => {
    it('should handle end event', async () => {
      dapiStream.endHandler();
      expect(dapiStream.stopReconnectTimeout).to.have.been.called();
      expect(dapiStream.emit).to.have.been.calledWith(DAPIStream.EVENTS.END);
      expect(dapiStream.stream).to.equal(null);
    });
  });
});
