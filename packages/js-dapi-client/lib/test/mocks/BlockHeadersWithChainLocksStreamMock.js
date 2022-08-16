const EventEmitter = require('events');

class BlockHeadersWithChainLocksStreamMock extends EventEmitter {
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
