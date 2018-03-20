const getLastBlock = SDK =>
  new Promise(((resolve, reject) => {
    const keys = Object.keys(SDK.Blockchain.blocks);
    keys.sort();
    const lastHeight = keys[keys.length - 1];
    if (lastHeight) {
      resolve(SDK.Blockchain.blocks[lastHeight]);
    } else {
      reject(new Error());
    }
  }));

module.exports = { getLastBlock };
