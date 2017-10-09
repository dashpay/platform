'use strict'

//blocks that can be considered part of the main chain/removed from this forkedChain
//probably 100 for production?
const matureHeight = 2;
const utils = require('./utils');

class ForkedChain {
    constructor(initBlock, mainChainPOW, mainChainTipHash) {
        this.POW = utils.getDifficulty(mainChainPOW);
        this.blocks = [initBlock];
        this.maturedBlocks = [];
        this.mainchainTipHash = mainChainTipHash;
        this.startTime = new Date().getTime();
    }

    isOrphan() {
        return this.mainchainTipHash != utils.getCorrectedHash(this.getHead().prevHash);
    }

    processMaturedBlocks() {
        if (this.blocks.length - 1 >= matureHeight) {
            let mBlock = this.blocks.shift();
            this.maturedBlocks.push(mBlock)
        }

        //todo: kill off expired blocks
    }

    addPOW(bits) {
        this.POW += utils.getDifficulty(bits);
    }

    addTip(block) {
        this.blocks.push(block);
        this.processMaturedBlocks();
        this.addPOW(block.bits);
    }

    addHead(block) {
        this.blocks.unshift(block);
        this.processMaturedBlocks();
        this.addPOW(block.bits);
    }

    isConnectedToTip(block) {
        return this.getTip().hash === utils.getCorrectedHash(block.prevHash)
    }

    isConnectedToHead(block) {
        return this.isOrphan() && this.getHead().hash == block.hash
    }

    getMaturedBlocks() {
        return this.maturedBlocks;
    }

    getTip() {
        return this.blocks[this.blocks.length - 1];
    }

    getHead() {
        return this.blocks[0];
    }

    getPOW() {
        return this.POW;
    }

    getForkHeight() {
        return this.blocks.length + this.maturedBlocks.length;
    }
}

module.exports = ForkedChain