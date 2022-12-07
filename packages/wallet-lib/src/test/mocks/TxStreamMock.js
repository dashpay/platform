const EventEmitter = require('events');
const TxStreamDataResponseMock = require('./TxStreamDataResponseMock');
const { waitOneTick } = require('../utils');

class TxStreamMock extends EventEmitter {
  constructor(sinon) {
    super();

    if (sinon) {
      sinon.spy(this, 'on');
      sinon.spy(this, 'removeListener');
      sinon.spy(this, 'emit');
      sinon.spy(this, 'cancel');
    }

    this.errored = false;
  }

  emit(event, data) {
    if (event === 'error') {
      this.errored = true;
    }
    super.emit(event, data);
  }

  async cancel() {
    await waitOneTick();
    if (!this.errored) {
      const err = new Error();
      err.code = 1;
      this.emit(TxStreamMock.EVENTS.error, err);
    }
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

  sendISLocks(isLocks) {
    this.emit(
      TxStreamMock.EVENTS.data,
      new TxStreamDataResponseMock(
        {
          instantSendLockMessages: isLocks.map((isLock) => isLock.toBuffer()),
        },
      ),
    );
  }

  sendMerkleBlock(merkleBlock) {
    this.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
      rawMerkleBlock: merkleBlock.toBuffer(),
    }));
  }

  finish() {
    this.emit(TxStreamMock.EVENTS.end);
  }

  errorHandler(e) {
    this.emit('error', e);
  }

  // eslint-disable-next-line class-methods-use-this
  retryOnError() {}
}

TxStreamMock.EVENTS = {
  cancel: 'cancel',
  data: 'data',
  end: 'end',
  error: 'error',
};

module.exports = TxStreamMock;
