const { clone } = require('khal');
const DashUtil = require('dash-util')
const Bitcore = require('bitcore-lib-dash');

module.exports = function(_b) {
    let _el = clone(_b);
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

        return new Bitcore.BlockHeader.fromObject(bh);
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
        return new Bitcore.BlockHeader.fromObject(bh);
    }
    return false;
};