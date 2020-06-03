const BaseTransporter = require('../BaseTransporter/BaseTransporter');

class RPCClient extends BaseTransporter {
  constructor(props) {
    super({ ...props, type: 'RPCClient' });
    this.ip = (props && props.ip) || '127.0.0.1';
  }
}
module.exports = RPCClient;
