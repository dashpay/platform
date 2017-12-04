const _config = require('../config');
const explorerPost = require('../Common/ExplorerHelper').explorerPost;
const message = require('bitcore-message-dash');
const Mnemonic = require('bitcore-mnemonic-dash');

const REFSDK = _config.useTrustedServer ? require('../Connector/trustedFactory.js') : require('../Connector/dapiFactory.js');

const options = { //no effect for dapi - using defaults
    verbose: false,
    errors: false,
    warnings: false,
    debug: false,
    DISCOVER: {
        INSIGHT_SEEDS: [
            /*{
                protocol: 'http',
                path: "insight-api-dash",
                base: "51.15.5.18",
                port: 3001
            },*/
            {
                protocol: 'https',
                path: "/insight-api-dash",
                base: "dev-test.dash.org",
                port: 443
            }
        ]
    }
};

REFSDK(options)
    .then(ready => {
        if (ready) {
            let mockUser = JSON.parse(require('../Accounts/User/mocks/registeredUser'));
            let _data = { owner: 'Alice', receiver: 'Bob', type: 'contactReq', txId: mockUser.txid }

            let mnemonic = new Mnemonic('jaguar paddle monitor scrub stage believe odor frown honey ahead harsh talk');
            let privKey = mnemonic.toHDPrivateKey().derive("m/1").privateKey;
            var _signature = message(JSON.stringify(_data)).sign(privKey);

            explorerPost(`/quorum`, {
                verb: 'add',
                qid: 0,
                data: _data,
                signature: _signature
            })
        }
        else {
            console.log("SDK not initialised")
        }
    })

//Override node promises (workaround debug issues)
global.Promise = require("bluebird");

// new Promise((resolve, reject) => {
//     breaksomething() //won't pause
// })