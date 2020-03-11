const BaseTransporter = require('./types/BaseTransporter/BaseTransporter');
const DAPIClient = require('./types/DAPIClient/DAPIClient');
const RPCClient = require('./types/RPCClient/RPCClient');
const ProtocolClient = require('./types/ProtocolClient/ProtocolClient');

const Transporters = {
  BaseTransporter,
  DAPIClient,
  RPCClient,
  ProtocolClient,
};
Transporters.getByName = require('./methods/getByName');
Transporters.resolve = require('./methods/resolve');
Transporters.validate = require('./methods/validate');

module.exports = Transporters;
