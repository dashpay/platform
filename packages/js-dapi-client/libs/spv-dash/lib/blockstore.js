'use strict'

//pvr: starting to question if levelup is the proper data structure
//no indexes and the headerchain possibly not large enough to justify using a db in the first place?
//not ideal to keep track of chain forks and a custom indexed/linked-list structure might be better suited?
const levelup = require('levelup'),
    utils = require('./utils')


var BlockStore = module.exports = function() {
    this.db = levelup('dash.chain',
        {
            db: require('memdown'),
            keyEncoding: 'utf8',
            valueEncoding: 'json'
        });

    this.Block = require('bitcore-lib-dash').BlockHeader;
    this.tipHash = null;
}

BlockStore.prototype.put = function(_header) {

    this.tipHash = utils.getCorrectedHash(_header._getHash());

    let self = this;

    return new Promise((resolve, reject) => {

        this.db.put(self.tipHash, _header, function(err) {
            if (!err) {
                resolve(self.tipHash);
            }
            else {
                //Todo update tiphash now incorrect
                reject(err)
            }
        })
    })
}

BlockStore.prototype.get = function(hash) {

    var self = this;

    return new Promise((resolve, reject) => {
        self.db.get(hash, function(err, data) {

            if (err && err.name == "NotFoundError") {
                resolve(null)
            }
            else if (err) {
                reject(err.message);
            }
            else {
                resolve(data);
            }

        })
    })
}

BlockStore.prototype.getTipHash = function() {
    return this.tipHash;
}

BlockStore.prototype.close = function(cb) {
    this.db.close();
}

BlockStore.prototype.isClosed = function() {
    return this.db.isClosed();
};

BlockStore.prototype.isOpen = function() {
    return this.db.isOpen();
};