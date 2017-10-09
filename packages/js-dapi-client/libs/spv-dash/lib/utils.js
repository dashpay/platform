'use strict'

const DashUtil = require('dash-util')
const bitcore = new require('bitcore-lib-dash');
const u256 = require('dark-gravity-wave-js/lib/u256')


var validProofOfWork = function(header) {
    var target = DashUtil.expandTarget(header.bits)
    var hash = header._getHash().reverse(); //replacement for require(bufer-reverse).reverse()
    return hash.compare(target) !== 1
}

module.exports = {

    //functions for getting normal (ie non reversed) merkleroot, hash and prevHash values
    //from bitcore-lib-dash.BlockHeader directly (this fn only temporary)
    //todo: add getHash(), getPrevHash() and getMerkleRoot() fns to bitcore and PR
    getCorrectedHash: function(reversedHashObj) {
        let clone = new Buffer(32);
        reversedHashObj.copy(clone)
        return clone.reverse().toString('hex');
    },
    createBlock: function(prev, bits) {
        var i = 0;
        var header = null;
        do {
            header = new bitcore.BlockHeader({
                version: 2,
                prevHash: prev ? prev._getHash() : DashUtil.nullHash,
                merkleRoot: DashUtil.nullHash,
                time: prev ? (prev.time + 1) : Math.floor(Date.now() / 1000),
                bits: bits,
                nonce: i++
            })
        } while (!validProofOfWork(header))
        return header
    },
    getDifficulty: function(target) {

        //TODO: temp hack to calculate difficulty (ie higer values for lower targets)
        //to replace with correct difficulty calculations
        return 1.0 / target;

        // var nSize = targetBits >>> 24;
        // var nWord = new u256();
        // nWord.u32[0] = targetBits & 0x007fffff;
        // if (nSize <= 3) {
        //     nWord = nWord.shiftRight(8 * (3 - nSize));
        // }
        // else {
        //     nWord = nWord.shiftLeft(8 * (nSize - 3));
        // }
        // return nWord.getCompact();
    },
    _normalizeHeader: function(_b) {
        let _el = JSON.parse(JSON.stringify(_b));
        let bh = {};
        if (_b.constructor.name == "BlockHeader") {
            return _b;
        }
        if (
            _el.hasOwnProperty("version") &&
            _el.hasOwnProperty("previousblockhash") &&
            _el.hasOwnProperty("merkleroot") &&
            _el.hasOwnProperty("time") &&
            _el.hasOwnProperty("bits") &&
            _el.hasOwnProperty("nonce")
        ) {
            if (!_el.previousblockhash) {
                _el.previousblockhash = new Buffer('0000000000000000000000000000000000000000000000000000000000000000', 'hex');
            }
            else {
                _el.previousblockhash = DashUtil.toHash(_el.previousblockhash);
            }
            _el.merkleroot = DashUtil.toHash(_el.merkleroot);
            bh = {
                version: _el.version,
                prevHash: _el.previousblockhash,
                merkleRoot: _el.merkleroot,
                time: _el.time,
                bits: parseInt(_el.bits, 16),
                nonce: _el.nonce
            };

            return new bitcore.BlockHeader.fromObject(bh);
        }
        if (
            _el.hasOwnProperty("version") &&
            _el.hasOwnProperty("prevHash") &&
            _el.hasOwnProperty("merkleRoot") &&
            _el.hasOwnProperty("time") &&
            _el.hasOwnProperty("bits") &&
            _el.hasOwnProperty("nonce")
        ) {
            if (!Buffer.isBuffer(_el.prevHash)) {
                _el.prevHash = DashUtil.toHash(_el.prevHash) || new Buffer('0000000000000000000000000000000000000000000000000000000000000000', 'hex');
                _el.merkleRoot = DashUtil.toHash(_el.merkleRoot);
                _el.bits = parseInt(_el.bits, 16);
            }
            bh = {
                version: _el.version,
                prevHash: _el.prevHash,
                merkleRoot: _el.merkleRoot,
                time: _el.time,
                bits: _el.bits,
                nonce: _el.nonce
            };
            return new bitcore.BlockHeader.fromObject(bh);
        }
        return false;
    }

}