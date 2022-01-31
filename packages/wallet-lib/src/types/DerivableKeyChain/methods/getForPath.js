const logger = require('../../../logger');

function getForPath(path, opts = {}) {
  if (path === undefined) throw new Error('Expect a valid path to derivate');
  const stringifiedPath = path.toString();
  logger.silly(`KeyChain.getForPath(${stringifiedPath})`);
  const isUsed = (opts && opts.isUsed !== undefined) ? opts.isUsed : false;
  const isWatched = (opts && opts.isWatched !== undefined) ? opts.isWatched : false;
  const isDerivable = ['HDPrivateKey', 'HDPublicKey'].includes(this.rootKeyType);
  if (!isDerivable && stringifiedPath !== '0') {
    throw new Error(`Wallet is not loaded from a mnemonic or a HDPrivateKey, impossible to derivate keys for path ${stringifiedPath}`);
  }

  let data;
  if (this.issuedPaths.has(stringifiedPath)) {
    data = this.issuedPaths.get(stringifiedPath);
    if (opts && opts.isWatched !== undefined && data.isWatched !== opts.isWatched) {
      data.isWatched = opts.isWatched;
    }
    if (opts && opts.isUsed !== undefined && data.isUsed !== opts.isUsed) {
      data.isUsed = opts.isUsed;
    }
    return data;
  }

  const key = (isDerivable) ? this.rootKey.derive(stringifiedPath) : this.getRootKey();

  data = {
    path: stringifiedPath,
    key,
    isUsed,
    isWatched,
    address: key.publicKey.toAddress(this.network),
    issuedTime: +new Date(),
  };

  this.issuedPaths.set(stringifiedPath, data);

  return data;
}

module.exports = getForPath;
