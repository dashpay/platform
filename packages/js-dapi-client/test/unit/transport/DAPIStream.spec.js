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
        message: ' CANCELED_ON_CLIENT',
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
    this.sinon.spy(dapiStream, 'removeListeners');
    this.sinon.spy(dapiStream, 'connect');
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
      expect(dapiStream.removeListeners).to.have.been.calledOnce();
      expect(dapiStream.addListeners).to.have.been.calledOnce();
      expect(dapiStream.reconnectTimeout).to.exist();
    });

    it('should cancel stream after timeout', async () => {
      await dapiStream.connect();
      timeoutCallback();
      expect(stream.cancel).to.have.been.calledOnce();
      expect(dapiStream.connect).to.have.been.calledTwice();
    });

    it('should update connect arguments', async () => {
      await dapiStream.connect();

      dapiStream.on(DAPIStream.EVENTS.BEFORE_RECONNECT, (updateArgs) => {
        updateArgs('newArg1', 'newArg2');
      });

      timeoutCallback();
      expect(dapiStream.connect).to.have.been.calledWith('newArg1', 'newArg2');
    });

    it('should handle error on reconnect', async () => {
      await dapiStream.connect();
      const errorEventPromise = new Promise((resolve) => {
        dapiStream.on('error', resolve);
      });

      const err = new Error('test error');
      dapiStream.streamFunction.throws(err);
      timeoutCallback();
      const emittedError = await errorEventPromise;
      expect(emittedError).to.equal(err);
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
      expect(dapiStream.endHandler).to.have.been.calledOnce();
    });

    it('should handle cancellation triggered by reconnect logic', async () => {
      await dapiStream.connect();
      timeoutCallback();
      expect(dapiStream.emit)
        .to.have.been.calledWith(DAPIStream.EVENTS.BEFORE_RECONNECT);
      expect(dapiStream.reconnectingAfterTimeout).to.be.false();
      expect(dapiStream.connect).to.have.been.calledTwice();
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
    it('should cancel stream', async () => {
      await dapiStream.connect();
      dapiStream.cancel();
      expect(dapiStream.stopReconnectTimeout).to.have.been.calledOnce();
      expect(stream.cancel).to.have.been.calledOnce();
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

  describe('#removeListeners', () => {

  });
});
