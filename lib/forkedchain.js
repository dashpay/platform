const utils = require('./utils');

class ForkedChain {
  constructor(initBlock, mainChainPOW) {
    this.POW = utils.getDifficulty(mainChainPOW);
    this.blocks = [initBlock];
    this.mainchainTipHash = initBlock.hash;
    this.startTime = new Date().getTime();
  }

  isOrphan() {
    return this.mainchainTipHash !== utils.getCorrectedHash(this.getHead().prevHash);
  }

  addPOW(bits) {
    this.POW += utils.getDifficulty(bits);
  }

  addTip(block) {
    this.blocks.push(block);
    this.addPOW(block.bits);
  }

  addHead(block) {
    this.blocks.unshift(block);
    this.addPOW(block.bits);
  }

  isConnectedToTip(block) {
    return this.getTip().hash === utils.getCorrectedHash(block.prevHash);
  }

  isConnectedToHead(block) {
    return this.isOrphan() && this.getHead().hash === block.hash;
  }

  getTip() {
    return this.blocks[this.blocks.length - 1];
  }

  getTipHash() {
    return this.getTip().hash;
  }

  getHead() {
    return this.blocks[0];
  }

  getPOW() {
    return this.POW;
  }

  getForkHeight() {
    return this.blocks.length;
  }

  pruneBlocks(len = 1) {
    this.blocks.splice(0, Math.max(this.blocks.length - len, 0));
  }
}

module.exports = ForkedChain;
