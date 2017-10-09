const should = require('should');
const _config = require('../config')

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

describe('Init DAPI-SDK', function() {
    it('should start the SDK', function() {
        global.SDK = REFSDK(options);
    });

    it('should have the right components', function() {
        should.exist(global.SDK);
        global.SDK.should.have.property('Accounts');
        global.SDK.should.have.property('Discover');
        global.SDK.should.have.property('Explorer');
        global.SDK.should.have.property('SPV');
    })
});
