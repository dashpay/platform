/* eslint-disable */
// TODO: Make this file pass linting!
// TODO: still just in POC code - optimisations needed to make it lib worthy
// All node/ pool sockets & communication, etc + bloom filters
// should be moved from here and handled in dapi - sdk Discovery services

const utils = require('./utils');

module.exports = isIncluded = function (localBlock, txHash) {
  return getMerkleProofs(localBlock.hash, localBlock.merkleRoot)
    .then((proofs) => {
      if (proofs.map(p => utils.getCorrectedHash(p)).includes(txHash)) {
        // coinbase tx only so
        // merkle root matches txHash so can do this check here
        // console.log('SPV VALIDTION SUCCESS')
        return true;
      }
      // console.log('SPV FAILED')
      return false;
    })
    .catch((err) => {
      console.log(err);
    });
};

getMerkleProofs = function (localBlockHash, localMerkleRoot, filterAddr = null) {
  return new Promise(((resolve, reject) => {
    // test.insight.dash.siampm.com
    const Peer = require('bitcore-p2p-dash').Peer;
    const Pool = require('bitcore-p2p-dash').Pool;
    const peer = new Peer({ host: 'test.insight.dash.siampm.com', port: '19999', network: 'testnet' }); // local: 127.0.0.1
    const pool = new Pool({ network: 'testnet' });

    peer.on('ready', () => {
      // not working
      if (filterAddr) {
        const BloomFilter = require('bitcore-p2p-dash').BloomFilter;
        const code = new Buffer(filterAddr, 'base64');
        var filter = BloomFilter.create(1, 0.1);
        filter.insert(code);
      }

      // Investigate pool/peer message mix, seems to be only way to get it to work
      pool.sendMessage(peer.messages.FilterLoad(filter));
      pool.sendMessage(peer.messages.GetData.forFilteredBlock(localBlockHash));
    });

    pool.on('peerheaders', (peer, message) => {
      console.log(`peerheaders:${JSON.stringify(message)}`);
    });

    pool.on('peermerkleblock', (peer, message) => {
      if (utils.getCorrectedHash(message.merkleBlock.header.merkleRoot) != utils.getCorrectedHash(localMerkleRoot)) {
        reject('merkle roots does not match on spv chain');
      } else {
        const bmp = require('bitcoin-merkle-proof');
        try {
          resolve(bmp.verify({
            flags: message.merkleBlock.flags,
            hashes: message.merkleBlock.hashes.map(h => Buffer.from(h, 'hex')),
            numTransactions: message.merkleBlock.numTransactions,
            merkleRoot: message.merkleBlock.header.merkleRoot,
          }));
        } catch (e) {
          reject(e);
        }
      }
    });

    // https://en.bitcoin.it/wiki/Protocol_documentation#Inventory_Vectors
    // Only types 0 - 4 but currently getting types of 15, 16, 7, 8 ??
    peer.on('inv', (message) => {
      // console.log(`inv: ${JSON.stringify(message.inventory)}`)

      if (message.inventory.filter(i => i.type == 3).length > 0) {
        console.log(`MERKLE BLOCK !!!!${JSON.stringify(message.inventory)}`); // not happening :(
      }
    });

    peer.on('tx', (message) => {
      console.log(`tx: ${message.transaction}`);
    });

    peer.on('addr', (message) => {
      console.log(`addr: ${JSON.stringify(message.addresses)}`);
    });

    peer.on('disconnect', () => {
      console.log('connection closed');
    });

    peer.on('error', (err) => {
      console.log(err);
    });

    pool.connect();
    peer.connect();
  }));
};

