const addBlock = blocks =>
  new Promise((async (resolve, reject) => {
    let blockHeaders = [];
    if (!Array.isArray(blocks)) {
      blockHeaders.push(blocks);
    } else if (blocks.length > 0) {
      blockHeaders = blocks;
    } else {
      resolve(false);
    }
    blockHeaders = blockHeaders.map(blockHeader => this.Blockchain.normalizeHeader(blockHeader));
    this.Blockchain.chain.addHeaders(blockHeaders, (error) => {
      if (error) {
        reject(error);
      }
      resolve(true);
    });
  }));

module.exports = { addBlock };
