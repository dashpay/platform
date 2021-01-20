const TxStreamMock = require('./TxStreamMock');
const TransportMock = require('./TransportMock');

module.exports = async function createAndAttachTransportMocksToWallet(wallet, sinon) {
  const txStreamMock = new TxStreamMock();
  const transportMock = new TransportMock(sinon, txStreamMock);

  // eslint-disable-next-line no-param-reassign
  wallet.transport = transportMock;

  const accountSyncPromise = wallet.getAccount();
  // Breaking the event loop to start wallet syncing
  await new Promise((resolve) => setTimeout(resolve, 0));
  // Emitting tx stream end to make wallet sync finish
  txStreamMock.emit(TxStreamMock.EVENTS.end);
  // Waiting for wallet to sync
  await accountSyncPromise;

  return { txStreamMock, transportMock };
};
