const should = require('should');
const DAPISDK = require('../index.js');
const options = {
    verbose: false,
    errors: true,
    warnings: true,
    debug: false,
    DISCOVER: {
        INSIGHT_SEEDS: [
            {
                protocol: 'http',
                path: "insight-api-dash",
                base: "51.15.5.18",
                port: 3001
            },
            {
                protocol: 'https',
                path: "insight-api-dash",
                base: "dev-test.dash.org",
                port: 443
            }
        ]
    }
};
describe('Init DAPI-SDK', function() {
    it('should start the SDK', function() {
        global.SDK = DAPISDK(options);
    });


    it('should have the right components', function() {
        should.exist(global.SDK);
        global.SDK.should.have.property('Accounts');
        global.SDK.should.have.property('Blockchain');
        global.SDK.should.have.property('Discover');
        global.SDK.should.have.property('Explorer');
    })
});