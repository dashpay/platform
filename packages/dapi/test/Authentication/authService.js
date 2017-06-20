'use strict'
require('../_before.js')

const Mnemonic = require('bitcore-mnemonic-dash');
const Message = require('bitcore-message-dash');
let mnemonic = new Mnemonic('jaguar paddle monitor scrub stage believe odor frown honey ahead harsh talk');
let privKey = mnemonic.toHDPrivateKey().derive("m/1/1495176227").privateKey;
let txId = 'cb1aa5d405c148a4990ff0035a6cd86cc73857ea57be3e49539cd8a9d0358315'

describe('Auth - EXPECTED TO FAIL PENDING DASHD HOOKUP ', function() {

    let challenge = dapi.authService.getChallenge();
    it('should get a challenge string', function() {
        (challenge.length > 0).should.equal.true;
    });

    it('should return a valid signature for challenge', function() {
        let nonce = 2;
        return dapi.authService.resolveChallenge('pierre', nonce, new Message(nonce.toString()).sign(privKey))
            .then(res => {
                res.should.be.true
            })
    });

    it('it should fail for invalid nonce', function() {
        let nonce = 0;
        return dapi.authService.resolveChallenge('pierre', nonce, new Message(nonce.toString()).sign(privKey))
            .then(res => {
                res.should.be.false
            })
    });
});