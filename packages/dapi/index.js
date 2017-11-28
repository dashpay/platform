const Dapi = require('./lib/dapi');
const config = require('./lib/config');
// let dapi = new Dapi(config);
const qDash = require('quorums-dash');

// QDEVTEMP
const dapiArr = [];

for (let i = 0; i < qDash.config.quorumSize * qDash.config.dapiMultiplicator; i++) {
  const newConfig = Object.assign({}, config);
  newConfig.server.port = 3000 + i;
  dapiArr.push(new Dapi(newConfig));
}
// QDEVTEMP END

