const Dapi = require('./lib/dapi');
//let dapi = new Dapi(require('./dapi.json'));
const qDash = require('quorums-dash')

//QDEVTEMP
let dapiArr = [];

for (let i = 0; i < qDash.config.quorumSize * qDash.config.dapiMultiplicator; i++) {
    let config = require('./dapi.json');
    config.server.port = 3000 + i;
    dapiArr.push(new Dapi(config))
}
//QDEVTEMP END



