const EventEmitter = require('events');

class BlockHeadersWithChainLocksStreamMock extends EventEmitter {
  constructor(sinon) {
    super();

    sinon.spy(this, 'on');
    sinon.spy(this, 'removeListener');
    sinon.spy(this, 'emit');
    sinon.spy(this, 'destroy');
    sinon.spy(this, 'removeAllListeners');
    sinon.spy(this, 'cancel');

    this.errored = false;
  }

  destroy(e) {
    this.emit('end');
    if (e) {
      this.emit('error', e);
    }
  }

  cancel() {
    if (!this.errored) {
      const err = new Error('CANCELED_ON_CLIENT');
      err.code = 1;
      this.emit('error', err);
    }
  }

  emit(event, data) {
    if (event === 'error') {
      this.errored = true;
    }
    super.emit(event, data);
  }

  /**
   * @param headers {BlockHeader[]}
   */
  sendHeaders(headers) {
    this.emit('data', {
      getBlockHeaders: () => ({
        getHeadersList() {
          return headers.map((header) => header.toBuffer());
        },
      }),
    });
  }
}

module.exports = BlockHeadersWithChainLocksStreamMock;
