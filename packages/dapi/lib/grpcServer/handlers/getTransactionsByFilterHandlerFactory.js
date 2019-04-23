const { RawTransaction } = require('@dashevo/dapi-grpc');

function getTransactionsByFilterHandlerFactory() {
  function getTransactionsByFilterHandler(call, callback) {
    const transaction = new RawTransaction();

    transaction.setBytes(Buffer.from('000000000000000', 'hex'));

    callback(null, transaction);
  }

  return getTransactionsByFilterHandler;
}

module.exports = getTransactionsByFilterHandlerFactory;
