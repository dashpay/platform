'use strict'
const spvchain = require('../libs/spv-dash/lib/spvchain'),
    merkleproof = require('../libs/spv-dash/lib/merkleproof')

var chain = null;

module.exports = {

    initChain: function(fileStream, chainType) {

        return new Promise((resolve, reject) => {
            chain = new spvchain(fileStream, chainType);

            chain.on('ready', function() {
                resolve(true);
            });
        })
    },

    getTipHash: function() {
        return chain.getTipHash();
    },

    addBlockHeaders: function(headers) {
        chain._addHeaders(headers);
        return chain.getChainHeight();
    },

    validateTx: function(blockHash, txHash) {

        return chain.getBlock(blockHash)
            .then(block => {
                return merkleproof(block, txHash);
            })
    },

    applyBloomFilter: function(addr) {
        //Todo
    }
};