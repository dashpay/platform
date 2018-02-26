// In memory storage of header chain
const levelup = require('levelup');
const utils = require('./utils');
const memdown = require('memdown');
const bitcore = require('bitcore-lib-dash');


const BlockStore = class {
  constructor() {
    this.db = levelup(memdown());
    this.block = bitcore.BlockHeader;
    this.tipHash = null;
  }

  put(header) {
    this.tipHash = utils.getCorrectedHash(header.getHash());

    const self = this;

    return new Promise((resolve, reject) => {
      this.db.put(self.tipHash, header, (err) => {
        if (!err) {
          resolve(self.tipHash);
        } else {
          // Todo update tiphash now incorrect
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
          resolve(data);
        }
      });
    });
  }

  getTipHash() {
    return this.tipHash;
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
};


module.exports = BlockStore;
