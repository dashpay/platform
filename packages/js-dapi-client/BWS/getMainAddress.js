const explorerGet = require('../Explorer/API/common/ExplorerHelper').explorerGet;
const axios = require('axios');
const moment = require('moment');
const Mnemonic = require('../util/mnemonic');
const Bitcoin = require('bitcoinjs-lib');

exports.getMainAddress = function (opts, noVerify, limit, reverse, rootKey, _mnemonic, _seed) {
    return new Promise(async function (resolve, reject) {
            // console.log(113, Mnemonic, Mnemonic.generateSeedFromMnemonic )
            let bip32Seed = _seed ? _seed : Mnemonic.generateSeedFromMnemonic(_mnemonic);
            // console.log(9, 'bip32Seed',  bip32Seed)
            let dashTestnet = {
                messagePrefix: '\x19DarkCoin Signed Message:\n',
                bip32: {public:70615939, private: 70615956},//Default was 50221816, but Copay use this one.
                pubKeyHash: 140,//140=y (139=y||x)
                scriptHash: 19,
                wif: 239,
                dustThreshold: 5460 // https://github.com/dashpay/dash/blob/v0.12.0.x/src/primitives/transaction.h#L144-L155
            };
            let dash = {
                messagePrefix: '\u0019DarkCoin Signed Message:\n',
                bip32: {public: 50221816, private: 50221772},
                pubKeyHash: 76,
                scriptHash: 16,
                wif: 204,
                dustThreshold: 5460
            };
            let bip32HDNode = Bitcoin.HDNode.fromSeedHex(bip32Seed, dashTestnet);

            let bip32RootKey = bip32HDNode.toBase58();

            let bip32RootAddress = bip32HDNode.getAddress();

            let pathDashTestnet = "m/44'/1'/0'/0/";
            let pathDashLivenet = "m/44'/5'/0'/0/";
            let rt = [bip32HDNode.derivePath(pathDashTestnet+0).getAddress()];

            let done = false;
            for (i=2; limit ? (i < limit) : true; i=i+20){

                for (j = i; j < i+20; j++){
                    let addy = bip32HDNode.derivePath(pathDashTestnet+j).getAddress();
                    let bal =  await SDK.Explorer.API.getBalance(addy);
                    if (!(bal > 0)) {
                        done = true;
                        break;
                    }
                    rt.push(addy);
                }
                if (done) {
                    break
                }
            }
            return resolve(rt)
        }
    );
};
