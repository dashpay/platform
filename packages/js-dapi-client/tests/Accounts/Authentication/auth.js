const should = require('should');
const Mnemonic = require('bitcore-mnemonic-dash')
require('../../_before.js');

let fundedAddr = 'yiBCPVWznF2nHDQD6H8wFWB8bhN8TKHFXc';
let username = 'pierre';
let mnemonic = new Mnemonic('jaguar paddle monitor scrub stage believe odor frown honey ahead harsh talk');
let privKey = mnemonic.toHDPrivateKey().derive("m/1").privateKey;
let authHeadAddr = mnemonic.toHDPrivateKey().derive("m/1/" + (new Date() / 1000)).privateKey.toAddress().toString(); //random new address

describe('AuthService', function() {


    it('should get a challenge string', function() {
        return SDK.Accounts.API.AuthService.getChallenge(`test_${new Date().getTime()}`)
            .then(challenge => {
                challenge.should.exist;
            })

    });

    // it('should create transaction on the blockchain with user object data', function() {

    //     return SDK.Accounts.User.create(fundedAddr, username, authHeadAddr, privKey)
    //         .then(res => {
    //             res.should.have.property('txid').with.lengthOf(64);
    //         })
    //         .catch(err => {
    //             console.log(err);
    //             should.fail;
    //         })
    // });
});