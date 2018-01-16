/* eslint-disable */
// TODO: Make this file pass linting!
const bcoin = require('bcoin').set('testnet');

// SPV chains only store the chain headers.
const chain = new bcoin.chain({
  db: 'leveldb',
  location: `${process.env.HOME}/spvchain`,
  spv: true,
});

const pool = new bcoin.pool({
  chain,
  spv: true,
  maxPeers: 8,
});

const walletdb = new bcoin.walletdb({ db: 'memory' });

pool.open().then(() => walletdb.open()).then(() => walletdb.create()).then((wallet) => {
  console.log('Created wallet with address %s', wallet.getAddress('base58'));

  // Add our address to the spv filter.
  pool.watchAddress(wallet.getAddress());

  // Connect, start retrieving and relaying txs
  pool.connect().then(() => {
    // Start the blockchain sync.
    pool.startSync();

    pool.on('tx', (tx) => {
      walletdb.addTX(tx);
    });

    wallet.on('balance', (balance) => {
      console.log('Balance updated.');
      console.log(bcoin.amount.btc(balance.unconfirmed));
    });
  });
});
