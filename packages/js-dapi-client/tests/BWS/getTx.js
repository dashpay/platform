const should = require('should');
require('../_before.js');

let _txid = '65d4f6369bf8a0785ae05052c86da4a57f76866805e3adadc82a13f7da41cbdf'

describe('BWS - getTx', function() {
    it('should return the transaction', async function(){
         let res = await SDK.BWS.getTx(_txid)
              res.should.be.a.Object();
              res.should.have.property('txid');
              res.txid.should.equal(_txid)
    });
});
