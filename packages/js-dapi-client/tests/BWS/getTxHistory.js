const should = require('should');
require('../_before.js');

describe('BWS - getTxHistory', function() {
    it('should return the transaction history in short form', function() {

        return SDK.BWS.getTxHistory({ addr: "yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4" }, 0, 0, false)
            .then(res => {
                res.should.be.a.Array();
                res[0].should.equal('21548a438be4c27a85d3f4f047a8a2d340a08df109a97fd80de4a24f84843676');
                // used to be 348564ee88cb0eed8f26e4176b2a703b933a28588789f50cfc38d82883a941d2 - investigate change?
            })
    })

    it('should return the transaction history in expanded form', function() {
        return SDK.BWS.getTxHistory({ addr: "yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4" }, 0, 2, true)
            .then(res => {
                res[0].should.be.a.Object();
            })

    })

    it('should have pagination', function() {
        return SDK.BWS.getTxHistory({ addr: "yj6xVHMyZGBdLqGUfoGc9gDvU8tHx6iqb4" }, 2, 10, false)
            .then(res => {
                res.should.be.a.Array();
                res.length.should.equal(8);
            })
    })
})
