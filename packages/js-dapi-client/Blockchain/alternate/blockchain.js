const levelup = require('levelup'); //required dpt for BSPVDash
const memdown = require('memdown');

const validParameters = function(params) {
    return typeof params.genesisHeader === 'object';
    // typeof params.shouldRetarget === 'function' &&
    // typeof params.calculateTarget === 'function' &&
    // typeof params.miningHash === 'function'
};
const Blockchain = function(params, options) {
    if (!params || !validParameters(params)) throw new Error('Invalid blockchain parameters');
    //We will store here all our block headers with hash as a key and blockheader as a value
    this.chain = levelup('dash.chain', { db: require('memdown') });
    //We will here store all our height as an indexed db with height as a key, and hash as the value
    this.height = levelup('dash.height', { db: require('memdown') });
    this.tip = -1;
    let self = this;
    return this.addHeader(params.genesisHeader).then(function() {
        return self;
    });
};
Blockchain.prototype.put = function(dbName, key, value) {
    let self = this;
    return new Promise(function(resolve, reject) {
        if (dbName === "chain" || dbName === "height") {
            self[dbName].put(key, value, function(err) {
                if (err) return resolve(false);
                return resolve(true);
            })
        }
    });
};
Blockchain.prototype.get = function(dbName, key) {
    let self = this;
    return new Promise(function(resolve, reject) {
        if (dbName === "chain" || dbName === "height") {
            self[dbName].get(key, function(err, result) {
                if (err) return resolve(false);
                return resolve(result);
            })
        }
    });
};
Blockchain.prototype.getTip = async function() {
    let self = this;
    return new Promise(function(resolve, reject) {
        let tipHeight = self.tip;
        console.log(self.tip);
        return resolve(self.getBlock(tipHeight));
    });
};
Blockchain.prototype.addHeader = async function(header) {
    let self = this;
    return new Promise(function(resolve, reject) {
        let hash = Buffer.from(header.hash);
        let height = header.height;
        let value = Buffer.from(JSON.stringify(header));
        let addHeader = self.put('chain', hash, value);
        let addHeight = self.put('height', height, hash);
        Promise
            .all([addHeader, addHeight])
            .then(function(result) {
                if (height > self.tip) {
                    self.tip = height;
                }
                return resolve(true);
            });
    });
};
Blockchain.prototype.getBlock = function(identifier) {
    if (identifier.constructor.name === "Buffer") {
        return this.getBlockByBufferedHash(identifier);
    }
    else if (identifier.constructor.name === "Number") {
        let height = identifier.toString();
        return this.getBlockByHeight(height);
    } else {
        let hash = identifier;
        return this.getBlockByHash(hash);
    }
};
Blockchain.prototype.getBlockByBufferedHash = function(bufferedHash) {
    let self = this;
    return new Promise(function(resolve, reject) {
        return self.get('chain', bufferedHash)
            .then(function(header) {
                header = JSON.parse(header.toString());
                return resolve(header);
            });
    });

};
Blockchain.prototype.getBlockByHash = function(hash) {
    let self = this;
    return new Promise(function(resolve, reject) {
        let bufferedHash = Buffer.from(hash);
        return self.get('chain', bufferedHash)
            .then(function(header) {
                header = JSON.parse(header.toString());
                return resolve(header);
            });
    })

}
Blockchain.prototype.getBlockByHeight = async function(height) {
    let self = this;
    return new Promise(function(resolve, reject) {
        return self
            .get('height', height)
            .then(function(_bufferedHash) {
                return self.get('chain', _bufferedHash)
                    .then(function(header) {
                        header = JSON.parse(header.toString());
                        return resolve(header);
                    });
            })
    });

}

module.exports = Blockchain;