const BaseTransporter = require('../BaseTransporter/BaseTransporter');

class RPCClient extends BaseTransporter {
  constructor(props) {
    super({ ...props, type: 'RPCClient' });
  }
}
module.exports = RPCClient;
