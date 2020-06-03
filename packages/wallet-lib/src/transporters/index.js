const BaseTransporter = require('./types/BaseTransporter/BaseTransporter');
const DAPIClientWrapper = require('./types/DAPIClientWrapper/DAPIClientWrapper');
const RPCClient = require('./types/RPCClient/RPCClient');
const ProtocolClient = require('./types/ProtocolClient/ProtocolClient');

const Transporters = {
  BaseTransporter,
  DAPIClientWrapper,
  RPCClient,
  ProtocolClient,
};
Transporters.getByName = require('./methods/getByName');
Transporters.resolve = require('./methods/resolve');
Transporters.validate = require('./methods/validate');

module.exports = Transporters;
