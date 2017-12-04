require('../../_before.js');
const should = require('should');
const Mnemonic = require('bitcore-mnemonic-dash')

let mnemonic = new Mnemonic('jaguar paddle monitor scrub stage believe odor frown honey ahead harsh talk');
let privKey = mnemonic.toHDPrivateKey().derive("m/1/1495176227").privateKey;
let txId = '89e3b3b4f62957ea94234293de9e01b3d509d5db67663c97d8369d018488bd12'

describe('Accounts - Login', function() {
    it('should authenticate with a server', function() {

        return SDK.Accounts.API.User.login(txId, privKey)
            .then(isAuthenticated => {
                isAuthenticated.should.be.true;
            })
            .catch(err => {
                console.log(err);
            })
    });
    it('should not auth with a server for invalid private key', function() {

        privKey = mnemonic.toHDPrivateKey().derive("m/1/99999").privateKey;

        return SDK.Accounts.API.User.login(txId, privKey)
            .then(isAuthenticated => {
                isAuthenticated.should.be.false;
            })
            .catch(err => {
                console.log(err);
            })
    });


});