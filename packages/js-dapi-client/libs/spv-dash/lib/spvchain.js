'use strict'
const utils = require('./utils');

const EventEmitter = require('events').EventEmitter,
    BlockStore = require('./blockstore'),
    bitcore = new require('bitcore-lib-dash'),
    inherits = require('inherits'),
    config = require('../config/config'),
    ForkedChain = require('./forkedchain')

var Blockchain = module.exports = function(fileStream, chainType) {
    this.store = new BlockStore();
    this.chainHeight = 0;
    this.forkedChains = [];
    this.POW = 0; //difficulty summed
    this.genesisHeader = null;
    this.ready = false;

    this._initStore(fileStream, chainType);
}
inherits(Blockchain, EventEmitter);

Blockchain.prototype.loadBlocksFromFile = function() {
    //Todo load

    //Todo confirm genesis match

    //Do re-scan/re-verification?

    self.emit('ready');
}

Blockchain.prototype._initStore = function(fileStream, chainType) {

    switch (chainType || 'lowdiff') {
        case 'testnet':
            this.genesisHeader = config.getTestnetGenesis();
            break;
        case 'livenet':
            break;
            this.genesisHeader = config.getLivenetGenesis();
        case 'lowdiff':
            this.genesisHeader = config.getLowDiffGenesis();
            break;
    }

    if (fileStream) {
        //loadBlocksFromFile() todo;
    }
    let self = this;

    if (!this.store.getTipHash()) {
        this.putStore(self.genesisHeader)
            .then(() => {
                this.ready = true;
                self.emit('ready');
            })
    }
    else {
        this.ready = true;
        self.emit('ready');
    }
}

Blockchain.prototype.getTipHash = function() {
    return this.store.getTipHash();
}

Blockchain.prototype.isChainReady = function() {
    return this.ready;
}

Blockchain.prototype.putStore = function(block) {
    this.POW += block.bits;
    this.chainHeight++;
    return this.store.put(block);
}

Blockchain.prototype.isValidBlock = function(header) {
    return header.validProofOfWork() &&
        header.validTimestamp &&
        header.getDifficulty() > 0; //todo: do some darkgravitywave check here or is this included in the validProofOfWork() check?
}

Blockchain.prototype.addCachedBlock = function(block) {
    let tipConnection = this.forkedChains.filter(fc => fc.isConnectedToTip(block))
    let headConnection = this.forkedChains.filter(fc => fc.isConnectedToHead(block))

    block.getDifficulty()

    if (tipConnection.length > 0) {
        tipConnection[0].addTip(block);
    }
    else if (headConnection.length > 0) {
        headConnection[0].addHead(block);
    }
    else {
        this.forkedChains.push(new ForkedChain(block, this.POW, this.store.getTipHash()));
    }
}

Blockchain.prototype.getBestFork = function() {
    let maxDifficulty = Math.max.apply(Math, this.forkedChains.map(f => f.getPOW()));
    return this.forkedChains.find(f => f.getPOW() == maxDifficulty)
}

Blockchain.prototype.processMaturedChains = function() {

    let bestChainMaturedBlocks = this.getBestFork().getMaturedBlocks();

    for (let i = 0; i < bestChainMaturedBlocks.length; i++) {
        this.putStore(bestChainMaturedBlocks.pop());
    }

    //todo: kill expired chains
}

Blockchain.prototype._addHeader = function(header) {

    if (!(header instanceof bitcore.BlockHeader)) {
        header = utils._normalizeHeader(header);
    }

    if (!this.isValidBlock(header)) {
        throw new Exception('Block does not conform to header consensus rules');
    }
    else {
        // console.log(`${header.bits} ${utils.getDifficulty(header.bits)}`)
        this.addCachedBlock(header);
        this.processMaturedChains();
    }
}

Blockchain.prototype._addHeaders = function(headers) {

    let self = this;
    headers.forEach(function(header) {
        self._addHeader(header);
    })
}

Blockchain.prototype.getChainHeight = function() {
    //Total chain = db committed chain + strongest fork/temp chain
    return this.chainHeight + this.getBestFork().getForkHeight();
}

Blockchain.prototype.getBlock = function(blockhash) {
    return this.store.get(blockhash)
        .then(blockInDB => {
            if (blockInDB) {
                return blockInDB
            }
            else {
                let blockInFork = this.getBestFork().blocks.filter(b => b.hash == blockhash);
                if (blockInFork.length == 1) {
                    return blockInFork[0];
                }
                else {
                    return null;
                }
            }
        })
}