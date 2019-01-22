const validator = require('validator');
const log = require('../log');


function verifyURL(url, component) {
  const valid = validator.isURL(url);
  if (!valid) {
    log.error(component, 'value is not valid. Valid url expected, found:', url);
  }
  return valid;
}

function verifyHost(host, component) {
  const valid = validator.isIP(host) || validator.isFQDN(host);
  if (!valid) {
    log.error(component, 'value is not valid. Valid host or ip address expected, found:', host);
  }
  return valid;
}

function verifyPort(port, component) {
  const valid = validator.isPort(port);
  if (!valid) {
    console.log(component, 'value is not valid. Valid port expected, found:', port);
  }
  return valid;
}

function verifyConfig(config) {
  let valid = true;
  valid = verifyURL(config.insightUri, 'INSIGHT_URI');
  valid = verifyHost(config.dashcore.p2p.host, 'DASHCORE_P2P_HOST') && valid;
  valid = verifyPort(config.dashcore.p2p.port, 'DASHCORE_P2P_PORT') && valid;
  valid = verifyHost(config.dashcore.rpc.host, 'DASHDRIVE_RPC_HOST') && valid;
  valid = verifyPort(config.dashcore.rpc.port, 'DASHDRIVE_RPC_PORT') && valid;
  valid = verifyHost(config.dashcore.zmq.host, 'DASHCORE_ZMQ_HOST') && valid;
  valid = verifyPort(config.dashcore.zmq.port, 'DASHCORE_ZMQ_PORT') && valid;
  valid = verifyHost(config.dashDrive.host, 'DASHDRIVE_RPC_HOST') && valid;
  valid = verifyPort(config.dashDrive.port, 'DASHDRIVE_RPC_PORT') && valid;
  valid = verifyPort(config.server.port.toString(), 'RPC_SERVER_PORT') && valid;

  if (!valid) {
    process.exit();
  }
}

module.exports = verifyConfig;
