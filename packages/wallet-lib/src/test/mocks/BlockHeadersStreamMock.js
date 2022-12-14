const EventEmitter = require('events');

class BlockHeadersStreamMock extends EventEmitter {
  constructor() {
    super();

    // onError minified events list
    this.f = [];
    // onEnd minified events list
    this.c = [];
    this.errored = false;
  }

  emit(event, data) {
    if (event === 'error') {
      this.errored = true;
    }
    super.emit(event, data);
  }

  cancel() {
    if (!this.errored) {
      const err = new Error();
      err.code = 1;
      this.emit(BlockHeadersStreamMock.EVENTS.error, err);
    }
  }

  end() {
    this.emit('end');
    this.removeAllListeners();
  }

  sendHeaders(headers) {
    this.emit(BlockHeadersStreamMock.EVENTS.data, {
      getBlockHeaders: () => ({
        getHeadersList: () => headers.map((header) => header.toBuffer()),
      }),
    });
  }

  finish() {
    this.emit(BlockHeadersStreamMock.EVENTS.end);
  }
}

BlockHeadersStreamMock.EVENTS = {
  cancel: 'cancel',
  data: 'data',
  end: 'end',
  error: 'error',
};

module.exports = BlockHeadersStreamMock;
