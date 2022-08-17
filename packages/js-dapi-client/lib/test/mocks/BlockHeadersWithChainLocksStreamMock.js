const EventEmitter = require('events');

class BlockHeadersWithChainLocksStreamMock extends EventEmitter {
  constructor(sinon) {
    super();

    sinon.spy(this, 'on');
    sinon.spy(this, 'removeListener');
    sinon.spy(this, 'emit');
    sinon.spy(this, 'destroy');
    sinon.spy(this, 'removeAllListeners');
  }

  destroy(e) {
    this.emit('error', e);
  }

  cancel() {
    this.emit('end');
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
