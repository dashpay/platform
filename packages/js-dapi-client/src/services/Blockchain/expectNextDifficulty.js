/* eslint no-underscore-dangle: 0, no-console: 0 */
const DGW = require('dark-gravity-wave-js');


const expectNextDifficulty = () => async () => new Promise((async (resolve) => {
  const lastBlock = await this.Blockchain.getLastBlock();
  console.log('Last', lastBlock.hash);
  if (lastBlock && lastBlock.height) {
    const lastHeight = lastBlock.height;
    console.log('height', lastHeight);
    let blockArr = [lastBlock];
    for (let i = lastHeight; i > (lastHeight - 24); i -= 1) {
      // TODO: Implement getLast25Blocks method
      // eslint-disable-next-line no-await-in-loop
      const block = await this.Blockchain.getBlock(i);
      if (block) {
        blockArr.push(block);
      } else {
        resolve(null);
      }
    }
    console.log(blockArr.length);
    if (blockArr.length === 25) {
      blockArr = blockArr.map(_h => ({
        height: _h.height,
        target: `0x${_h.bits}`,
        timestamp: _h.time,
      }));
      const nextbits = DGW.darkGravityWaveTargetWithBlocks(blockArr).toString(16);
      resolve(nextbits);
    }
    resolve(null);
  }
  resolve(null);
}));

module.exports = { expectNextDifficulty };
