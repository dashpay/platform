const {clone}=require('khal');
const DashUtil = require('dash-util')
const Bitcore = require('bitcore-lib-dash');

module.exports = function(_b){
    let _el = clone(_b);
    let bh = {};
    if(_b.constructor.name=="BlockHeader"){
        return _b;
    }
    if(
        _el.hasOwnProperty("version") &&
        _el.hasOwnProperty("previousblockhash") &&
        _el.hasOwnProperty("merkleroot") &&
        _el.hasOwnProperty("time") &&
        _el.hasOwnProperty("bits") &&
        _el.hasOwnProperty("nonce")
    ){
        bh = {
            version:_el.version,
            prevHash:DashUtil.toHash(_el.previousblockhash),
            merkleRoot:DashUtil.toHash(_el.merkleroot),
            time:_el.time,
            bits:parseInt(_el.bits,16),
            nonce:_el.nonce
        };

        return new Bitcore.BlockHeader.fromObject(bh);
    }
    if(
        _el.hasOwnProperty("version") &&
        _el.hasOwnProperty("prevHash") &&
        _el.hasOwnProperty("merkleRoot") &&
        _el.hasOwnProperty("time") &&
        _el.hasOwnProperty("bits") &&
        _el.hasOwnProperty("nonce")
    ){
        bh = {
            version:_el.version,
            prevHash:DashUtil.toHash(_el.prevHash),
            merkleRoot:DashUtil.toHash(_el.merkleRoot),
            time:_el.time,
            bits:parseInt(_el.bits,16),
            nonce:_el.nonce
        };
        return new Bitcore.BlockHeader.fromObject(bh);
    }
    return false;
};