const should = require('should');
require('../_before.js');

let _txid = '3550dd0f57c90304e8d97990444e7dd156cafd1339eb9cbf3540f1752b54be3a'

describe('BWS - getTx', function() {
    it('should return the transaction', async function() {
        let res = await SDK.BWS.getTx(_txid)
        res.should.be.a.Object();
        res.should.have.property('txid');
        res.txid.should.equal(_txid)
    });
});
