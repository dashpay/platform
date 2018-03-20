const getBlock = () => async height => new Promise((async (resolve) => {
  if (!this.Blockchain.blocks.height) {
    const block = await this.Blockchain.blocks[height];
    resolve(block);
  }
}));

module.exports = { getBlock };
