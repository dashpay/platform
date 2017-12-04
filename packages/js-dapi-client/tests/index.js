const should = require('should');
const _config = require('../config')

const REFSDK = _config.useTrustedServer ? require('../Connector/trustedFactory.js') : require('../Connector/dapiFactory.js');

describe('Init DAPI-SDK', function() {
    it('should have the right components', function() {
        return REFSDK()
            .then(success => {
                should.exist(global.SDK);
                global.SDK.should.have.property('Accounts');
                global.SDK.should.have.property('Discover');
                global.SDK.should.have.property('Explorer');
                global.SDK.should.have.property('Quorum');
                global.SDK.should.have.property('SPV');
            })
    })

});
