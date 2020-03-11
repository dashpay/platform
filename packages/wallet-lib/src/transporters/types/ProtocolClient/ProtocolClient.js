const BaseTransporter = require('../BaseTransporter/BaseTransporter');

class ProtocolClient extends BaseTransporter {
  constructor(props) {
    super({ ...props, type: 'ProtocolClient' });
  }
}
module.exports = ProtocolClient;
