const EventEmitter = require('events');
const TxStreamDataResponseMock = require('./TxStreamDataResponseMock');

class TxStreamMock extends EventEmitter {
  constructor() {
    super();

    // onError minified events list
    this.f = [];
    // onEnd minified events list
    this.c = [];
  }

  cancel() {
    const err = new Error();
    err.code = 2;
    this.emit(TxStreamMock.EVENTS.error, err);
  }

  end() {
    this.emit('end');
    this.removeAllListeners();
  }

  sendTransactions(transactions) {
    this.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
      rawTransactions: transactions.map((tx) => tx.toBuffer()),
    }));
  }

  finish() {
    this.emit(TxStreamMock.EVENTS.end);
  }
}

TxStreamMock.EVENTS = {
  cancel: 'cancel',
  data: 'data',
  end: 'end',
  error: 'error',
};

module.exports = TxStreamMock;
