require('../../_before.js');
const should = require('should');

describe('Insight-API - getStatus', function() {
    it('should return the valid getStatus ', function() {
        return SDK.Explorer.API.getStatus()
            .then(status => {
                status.should.have.property('info');
                status.info.should.have.property('version');
                status.info.should.have.property('protocolversion');
                status.info.should.have.property('blocks');
                status.info.should.have.property('timeoffset');
                status.info.should.have.property('connections');
                status.info.should.have.property('proxy');
                status.info.should.have.property('difficulty');
                status.info.should.have.property('testnet');
                status.info.should.have.property('relayfee');
                status.info.should.have.property('errors');
                // status.info.should.have.property('network');
            })
            .catch(e => {
                console.log(e);
            })

    });

});