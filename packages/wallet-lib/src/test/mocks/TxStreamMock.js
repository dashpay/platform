const EventEmitter = require('events');

class TxStreamMock extends EventEmitter {
  cancel() {
    const err = new Error();
    err.code = 2;
    this.emit(TxStreamMock.EVENTS.error, err);
  }

  end() {
    this.emit('end');
    this.removeAllListeners();
  }
}

TxStreamMock.EVENTS = {
  cancel: 'cancel',
  data: 'data',
  end: 'end',
  error: 'error',
};

module.exports = TxStreamMock;
