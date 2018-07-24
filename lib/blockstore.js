const levelup = require('levelup');
const memdown = require('memdown');
const bitcore = require('bitcore-lib-dash');

class BlockStore {
  constructor() {
    this.db = levelup(memdown(), {
      keyAsBuffer: false,
      valueAsBuffer: false,
      valueEncoding: 'json',
    });
    this.block = bitcore.BlockHeader;
  }

  put(header) {
    return new Promise((resolve, reject) => {
      this.db.put(header.hash, JSON.stringify(header.toObject()), (err) => {
        if (!err) {
          resolve(header.hash);
        } else {
          reject(err);
        }
      });
    });
  }

  get(hash) {
    const self = this;

    return new Promise((resolve, reject) => {
      self.db.get(hash, (err, data) => {
        if (err && err.name === 'NotFoundError') {
          resolve(null);
        } else if (err) {
          reject(err.message);
        } else {
          resolve(JSON.parse(data.toString()));
        }
      });
    });
  }

  close() {
    this.db.close();
  }

  isClosed() {
    return this.db.isClosed();
  }

  isOpen() {
    return this.db.isOpen();
  }
}


module.exports = BlockStore;
