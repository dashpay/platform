const Mnemonic = {
    generateMnemonicAndSeed: function(lang = 'english', passphrase = '') {
        const bip39 = require('bip39');

        let strenght = 128;
        let rng = null;//random number generator
        let wordlist = bip39.wordlists[lang];

        let mnemonic = bip39.generateMnemonic(strenght, rng, wordlist);
        let valid = bip39.validateMnemonic(mnemonic);
        if (!valid)
            return Error('Invalid mnemonic!');
        let entropy = bip39.mnemonicToEntropy(mnemonic);
        let seed = bip39.mnemonicToSeedHex(mnemonic, passphrase);

        return {
            bits: strenght,
            language: lang,
            seed: seed,
            entropy: entropy,
            phrase: mnemonic,
            passphrase: passphrase

        }
    },
    generateSeedFromMnemonic: function(mnemonic, passphrase = '') {
        const bip39 = require('bip39');
        let seed = bip39.mnemonicToSeedHex(mnemonic, passphrase);
        return seed;//return HEX seed
    }
};
module.exports = Mnemonic;