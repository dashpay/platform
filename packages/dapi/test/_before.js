if (!global.hasOwnProperty('dapi')) {
    const Dapi = require('../lib/dapi.js');
    global.dapi = new Dapi(require('../dapi.json'));
    const should = require('should');
}