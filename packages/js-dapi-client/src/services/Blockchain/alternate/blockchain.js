const levelup = require('levelup');
const memdown = require('memdown');

const validParameters = params => typeof params.genesisHeader === 'object';

const Blockchain = (params) => {
  if (!params || !validParameters(params)) throw new Error('Invalid blockchain parameters');
  // Block headers with hash as a key and blockheader as a value
  this.chain = levelup('dash.chain', { db: memdown });
  // Height as an indexed db with height as a key, and hash as the value
  this.height = levelup('dash.height', { db: memdown });
  this.tip = -1;
  return this.addHeader(params.genesisHeader).then(() => this);
};

Blockchain.prototype.put = (dbName, key, value) => new Promise(((resolve) => {
  if (dbName === 'chain' || dbName === 'height') {
    this[dbName].put(key, value, (err) => {
      if (err) {
        resolve(false);
      }
      resolve(true);
    });
  }
}));

Blockchain.prototype.get = (dbName, key) => new Promise(((resolve) => {
  if (dbName === 'chain' || dbName === 'height') {
    this[dbName].get(key, (err, result) => {
      if (err) {
        resolve(false);
      }
      resolve(result);
    });
  }
}));

Blockchain.prototype.getTip = async () => new Promise(((resolve) => {
  const tipHeight = this.tip;
  return resolve(this.getBlock(tipHeight));
}));

Blockchain.prototype.addHeader = async header => new Promise(((resolve) => {
  const hash = Buffer.from(header.hash);
  const { height } = header;
  const value = Buffer.from(JSON.stringify(header));
  const addHeader = this.put('chain', hash, value);
  const addHeight = this.put('height', height, hash);
  Promise
    .all([addHeader, addHeight])
    .then(() => {
      if (height > this.tip) {
        this.tip = height;
      }
      return resolve(true);
    });
}));

Blockchain.prototype.getBlock = (identifier) => {
  if (identifier.constructor.name === 'Buffer') {
    return this.getBlockByBufferedHash(identifier);
  } else if (identifier.constructor.name === 'Number') {
    const height = identifier.toString();
    return this.getBlockByHeight(height);
  }
  const hash = identifier;
  return this.getBlockByHash(hash);
};

Blockchain.prototype.getBlockByBufferedHash = bufferedHash =>
  new Promise((resolve => this.get('chain', bufferedHash)
    .then((header) => {
      const jsonHeader = JSON.parse(header.toString());
      return resolve(jsonHeader);
    })));

Blockchain.prototype.getBlockByHash = hash => new Promise(((resolve) => {
  const bufferedHash = Buffer.from(hash);
  return this.get('chain', bufferedHash)
    .then((header) => {
      const jsonHeader = JSON.parse(header.toString());
      return resolve(jsonHeader);
    });
}));

Blockchain.prototype.getBlockByHeight = async height =>
  new Promise((resolve => this
    .get('height', height)
    .then(_bufferedHash => this.get('chain', _bufferedHash)
      .then((header) => {
        const jsonHeader = JSON.parse(header.toString());
        return resolve(jsonHeader);
      }))));

module.exports = { Blockchain };
