if (!global.hasOwnProperty('dapi')) {
  const Dapi = require('../lib/dapi.js');
  global.dapi = new Dapi(require('../lib/config'));
  const should = require('should');
}
