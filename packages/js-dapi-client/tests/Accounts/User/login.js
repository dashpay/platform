require('../../_before.js');
const should = require('should');
const Mnemonic = require('bitcore-mnemonic-dash')

let mnemonic = new Mnemonic('jaguar paddle monitor scrub stage believe odor frown honey ahead harsh talk');
let privKey = mnemonic.toHDPrivateKey().derive("m/1/1495176227").privateKey;
let txId = 'cb1aa5d405c148a4990ff0035a6cd86cc73857ea57be3e49539cd8a9d0358315'

describe('Accounts - Login', function() {
    it('should authenticate with a server', function() {

        return SDK.Accounts.API.User.login(txId, privKey)
            .then(isAuthenticated => {
                res.should.be.true;
            })
            .catch(err => {
                console.log(err);
            })
    });
    it('should not auth with a server for invalid private key', function() {

        privKey = mnemonic.toHDPrivateKey().derive("m/1/99999").privateKey;

        return SDK.Accounts.API.User.login(txId, privKey)
            .then(isAuthenticated => {
                res.should.be.false;
            })
            .catch(err => {
                console.log(err);
            })
    });


});