const bip39 = require('bip39');

const generateMnemonicAndSeed = (lang = 'english', passphrase = '') => {
  const strength = 128;
  const rng = null;// random number generator
  const wordlist = bip39.wordlists[lang];

  const mnemonic = bip39.generateMnemonic(strength, rng, wordlist);
  const valid = bip39.validateMnemonic(mnemonic);
  if (!valid) { throw new Error('Invalid mnemonic!'); }
  const entropy = bip39.mnemonicToEntropy(mnemonic);
  const seed = bip39.mnemonicToSeedHex(mnemonic, passphrase);

  return {
    bits: strength,
    language: lang,
    seed,
    entropy,
    phrase: mnemonic,
    passphrase,
  };
};

const generateSeedFromMnemonic = (mnemonic, passphrase = '') => {
  const seed = bip39.mnemonicToSeedHex(mnemonic, passphrase);
  return seed;// return HEX seed
};

module.exports = {
  generateMnemonicAndSeed,
  generateSeedFromMnemonic,
};
