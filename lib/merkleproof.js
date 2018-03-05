const utils = require('./utils');
const { bitcoreP2p } = require('bitcore-p2p-dash');
const bmp = require('bitcoin-merkle-proof');

const { BloomFilter } = bitcoreP2p.BloomFilter;


function getMerkleProofs(localBlockHash, localMerkleRoot, filterAddr = null) {
  return new Promise(((resolve, reject) => {
    const peer = new bitcoreP2p.Peer({
      host: 'test.insight.dash.siampm.com',
      port: '19999',
      network: 'testnet',
    });
    const pool = new bitcoreP2p.Pool({ network: 'testnet' });

    peer.on('ready', () => {
      // not working
      if (filterAddr) {
        const code = new Buffer.From(filterAddr, 'base64');
        const filter = BloomFilter.create(1, 0.1);
        filter.insert(code);
        pool.sendMessage(peer.messages.FilterLoad(filter));
      }

      pool.sendMessage(peer.messages.GetData.forFilteredBlock(localBlockHash));
    });

    pool.on('peerheaders', (fromPeer, message) => {
      console.log(`peerheaders:${JSON.stringify(message)}`);
    });

    pool.on('peermerkleblock', (fromPeer, message) => {
      if (utils.getCorrectedHash(message.merkleBlock.header.merkleRoot)
    !== utils.getCorrectedHash(localMerkleRoot)) {
        reject(new Error('merkle roots does not match on spv chain'));
      } else {
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

      if (message.inventory.filter(i => i.type === 3).length > 0) {
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
}

function isIncluded(localBlock, txHash) {
  return getMerkleProofs(localBlock.hash, localBlock.merkleRoot)
    .then((proofs) => {
      if (proofs.map(p => utils.getCorrectedHash(p)).includes(txHash)) {
        // coinbase tx only so
        // merkle root matches txHash so can do this check here
        // console.log('SPV VALIDTION SUCCESS')
        return true;
      }
      return false;
    })
    .catch(err => err);
}


module.exports = isIncluded;
